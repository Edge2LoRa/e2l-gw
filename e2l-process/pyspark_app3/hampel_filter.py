# Import necessary libraries
import findspark
from pyspark import SparkContext, SparkConf
from pyspark.streaming import StreamingContext
from pyspark.sql import SparkSession, functions as F, Window
from pyspark.sql.types import (
    StructType,
    StructField,
    StringType,
    IntegerType,
    DoubleType,
)
from pyspark.sql.functions import to_unix_timestamp, col, lit
from mqtt import MQTTUtils
import os
import json
import time
import logging
import base64 as b64
import paho.mqtt.client as mqtt

# Initialize Spark
findspark.init()
spark_conf = SparkConf().setAppName("e2l-process")
sc = SparkContext(conf=spark_conf)
sc.setLogLevel("ERROR")
spark = SparkSession(sc)
ssc = StreamingContext(sc, 5)  # 5-sec batch interval

# Set up env variables
log = logging.getLogger(__name__)
DEBUG = os.getenv("DEBUG", False)
DEBUG = True if DEBUG == "1" else False
if DEBUG:
    from dotenv import load_dotenv

    load_dotenv()
    log.setLevel(logging.INFO)
else:
    log.setLevel(logging.INFO)


# Function to check environment variables
def check_env_vars() -> bool:
    env_vars = [
        "MQTT_USERNAME",
        "MQTT_PASSWORD",
        "MQTT_BROKER_HOST",
        "MQTT_BROKER_PORT",
        "MQTT_TOPIC",
        "MQTT_TOPIC_OUTPUT",
        "WINDOW_LENGTH",
        "SLIDE_INTERVAL",
        "WINDOW_SIZE_HAMPEL",
    ]
    for var in env_vars:
        if os.getenv(var) is None:
            log.error(f"{var} not set")
            exit(1)
        if "_PORT" in var:
            if not os.getenv(var).isnumeric():
                log.error(f"{var} must be numeric")
                exit(1)
            if int(os.getenv(var)) < 0 or int(os.getenv(var)) > 65535:
                log.error(f"{var} must be between 0 and 65535")
                exit(1)
    return True


# Function to perform the Hampel filter
def hampel_filter(data, window_size, n_sigma):
    window = (
        Window.partitionBy("dev_addr")
        .orderBy("timestamp")
        .rangeBetween(-window_size, 0)
    )
    data = data.withColumn(
        "median", F.expr("percentile_approx(soil_temp, 0.5)").over(window)
    )
    data = data.withColumn(
        "mad", F.expr("percentile_approx(abs(soil_temp - median), 0.5)").over(window)
    )
    data = data.withColumn("threshold", n_sigma * 1.4826 * F.col("mad"))
    data = data.withColumn(
        "is_outlier", F.abs(F.col("soil_temp") - F.col("median")) > F.col("threshold")
    )
    # print("Received Data:", data.count())
    outliers = data.filter("is_outlier")
    # print("Outlier detected: ", outliers.count())
    print([row.timestamp for row in data.select("timestamp").collect()])
    if outliers.count() > 0:
        outliers_detected = outliers.select("dev_addr", "fcnt").collect()
        json_outliers = [
            {"devaddr": row.dev_addr, "fcnt": row.fcnt} for row in outliers_detected
        ]
        json_payload = {
            "devaddr": "0036D020",
            "aggregated_data": json_outliers,
            "devaddrs": [row.dev_addr for row in data.select("dev_addr").collect()],
            "fcnts": [row.fcnt for row in data.select("fcnt").collect()],
            "timestamps": [row.timestamp for row in data.select("timestamp").collect()],
            "timestamp_pub": int(time.time() * 1000),
        }
        publish_output_spark(json_payload)


# Function to publish output via MQTT
def publish_output_spark(payload):
    client = mqtt.Client(mqtt.CallbackAPIVersion.VERSION2, client_id="publisher_output")
    client.username_pw_set(os.getenv("MQTT_USERNAME"), os.getenv("MQTT_PASSWORD"))
    client.connect(os.getenv("MQTT_BROKER_HOST"), int(os.getenv("MQTT_BROKER_PORT")))
    client.loop_start()
    json_reading = json.dumps(payload)
    print("sto publicando")
    client.publish(os.getenv("MQTT_TOPIC_OUTPUT"), json_reading)
    client.disconnect()
    client.loop_stop()


# Define schema for incoming data
schema = StructType(
    [
        StructField("dev_eui", StringType(), True),
        StructField("dev_addr", StringType(), True),
        StructField("fcnt", IntegerType(), True),
        StructField("timestamp", StringType(), True),
        StructField("frequency", DoubleType(), True),
        StructField("data_rate", StringType(), True),
        StructField("coding_rate", StringType(), True),
        StructField("gtw_id", StringType(), True),
        StructField("gtw_channel", IntegerType(), True),
        StructField("gtw_rssi", IntegerType(), True),
        StructField("gtw_snr", DoubleType(), True),
        StructField("payload", StringType(), True),
    ]
)


# Process readings
def process_readings(rdd):
    batch_df = spark.read.schema(schema).json(rdd)
    extract_temp = F.udf(
        lambda payload: json.loads(b64.b64decode(payload).decode("utf-8"))[0],
        DoubleType(),
    )
    batch_df = batch_df.withColumn("soil_temp", extract_temp(col("payload")))
    # timestamp = batch_df.select("timestamp").withColumn(
    #     "timestamp",
    #     unix_timestamp("timestamp", format="yyyy-MM-dd'T'HH:mm:ss.SSS'Z'").cast(
    #         "timestamp"
    #     ),
    # )
    input_data = batch_df.select("dev_addr", "fcnt", "timestamp", "soil_temp")
    input_data = input_data.withColumn(
        "timestamp",
        to_unix_timestamp(
            timestamp="timestamp", format=lit("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'")
        ),
    )
    window_size = int(os.getenv("WINDOW_SIZE_HAMPEL"))
    n_sigma = 1.0
    hampel_filter(input_data, window_size, n_sigma)


# Set up MQTT stream
broker_address = (
    "tcp://" + os.getenv("MQTT_BROKER_HOST") + ":" + str(os.getenv("MQTT_BROKER_PORT"))
)


readings = MQTTUtils.createStream(
    ssc,
    broker_address,
    os.getenv("MQTT_TOPIC"),
    os.getenv("MQTT_USERNAME"),
    os.getenv("MQTT_PASSWORD"),
)

# Process each RDD in the stream
windowed_readings = readings.window(
    int(os.getenv("WINDOW_LENGTH")), int(os.getenv("SLIDE_INTERVAL"))
)

windowed_readings.foreachRDD(process_readings)

# Start the Spark Streaming context
ssc.start()
ssc.awaitTermination()
