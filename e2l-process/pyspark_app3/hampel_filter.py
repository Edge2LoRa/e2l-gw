
import findspark
import paho.mqtt.client as mqtt
import json
import time
import logging
import os
import base64 as b64
from pyspark import SparkContext,  SparkConf
from pyspark.streaming import StreamingContext
from pyspark.sql import SparkSession
from pyspark.sql import functions as F
from pyspark.sql.functions import *
from pyspark.sql.types import *
from pyspark.sql import Window
#eseguire l'app cosi: spark-submit --packages org.apache.bahir:spark-streaming-mqtt_2.12:2.4.0 subscribe_rssi.py
#oppure aggiungere il package into spark-defaults.conf
from mqtt import MQTTUtils 



DEBUG = os.getenv("DEBUG", False)
DEBUG = True if DEBUG == "1" else False
if DEBUG:
    from dotenv import load_dotenv

    load_dotenv()
    #logging.basicConfig(level=logging.INFO)
else:
    #logging.basicConfig(level=logging.INFO)
    pass

log = logging.getLogger(__name__)


def check_env_vars() -> bool:
    env_vars = [
        "MQTT_USERNAME",
        "MQTT_PASSWORD",
        "MQTT_BROKER_HOST",
        "MQTT_BROKER_PORT",
        "MQTT_TOPIC",
        "MQTT_TOPIC_OUTPUT",
        "WINDOW_SIZE_HAMPEL"
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

#function that perform the Hampel filter
def hampel_filter(data,window_size,n_sigma):
    
    #define the window --> here the window is defined based on the timestamp field of the packet and not based on the arrival time of the packet
    #questo vuol dire che la finestra temporale per l'hampel filter è definita sul timestamp presente nel pacchetto e non in base al tempo che ricevo i pacchetti
    window = Window.partitionBy("dev_addr").orderBy("timestamp").rangeBetween(-window_size,0)

    #calculate the median for temp values over the window specified
    data = data.withColumn("median", F.expr('percentile_approx(soil_temp, 0.5)').over(window))

    #calculate the median absolute deviations
    data = data.withColumn("mad", F.expr("percentile_approx(abs(soil_temp - median), 0.5)").over(window))

    # calculate threshold for outlier detection
    data = data.withColumn("threshold", n_sigma * 1.4826 * F.col("mad"))
    
    #detect outliers
    data = data.withColumn("is_outlier", F.abs(F.col("soil_temp") - F.col("median")) > F.col("threshold"))

    #data.show(truncate=False)

    print("Received Data :",data.count())
    
    #take the outliers from data
    outliers = data.filter("is_outlier")
    #outliers.select("id", "nodeid", "timestamp", "soil_temp", "parsed_ts").show(truncate=False)
    
    print("Outlier detected: ", outliers.count())
    
    
    if outliers.count() > 0:
        # Take the id of outliers and convert them into a list
        outliers_detected = outliers.select("dev_addr", "fcnt").collect()
        
        # Prepare JSON payload
        json_outliers = [{"dev_addr": row.dev_addr, "fcnt": row.fcnt} for row in outliers_detected]
        json_payload = {
            "outliers": json_outliers,
            "timestamp_pub": int(time.time() * 1000)
        }
        
        # Publish output
        publish_output_spark(json_payload)


def publish_output_spark(payload):
    
    client = mqtt.Client(mqtt.CallbackAPIVersion.VERSION2, client_id="publisher_output")
    client.username_pw_set(os.getenv("MQTT_USERNAME"), os.getenv("MQTT_PASSWORD"))

    client.connect(os.getenv("MQTT_BROKER_HOST"), int(os.getenv("MQTT_BROKER_PORT")))


    client.loop_start()

    json_reading = json.dumps(payload)
    
    print("sto publicando:", json_reading)
    client.publish(os.getenv("MQTT_TOPIC_OUTPUT"),json_reading)

    client.disconnect()
    client.loop_stop()


if __name__ == "__main__":
    findspark.init()

    spark_conf= SparkConf()
    spark_conf.setAppName("e2l-process")
    
    
    sc = SparkContext(conf=spark_conf)
    sc.setLogLevel("ERROR")

    spark = SparkSession(sc)
    ssc = StreamingContext(sc, 30) #30-sec batch interval
    
     # broker parameters
    broker_address = (
        "tcp://"
        + os.getenv("MQTT_BROKER_HOST")
        + ":"
        + str(os.getenv("MQTT_BROKER_PORT"))
    )

    # Using MQTTUtils to receive the stream mqtt
    readings = MQTTUtils.createStream(
        ssc,
        broker_address,
        os.getenv("MQTT_TOPIC"),
        os.getenv("MQTT_USERNAME"),
        os.getenv("MQTT_PASSWORD"),
    )

    
    schema = StructType([
        StructField("dev_eui", StringType(), True),
        StructField("dev_addr", StringType(), True),
        StructField("fcnt", IntegerType(), True),
        StructField("timestamp", IntegerType(), True),
        StructField("frequency", DoubleType(), True),
        StructField("data_rate", StringType(), True),
        StructField("coding_rate", StringType(), True),
        StructField("gtw_id", StringType(), True),
        StructField("gtw_channel", IntegerType(), True),
        StructField("gtw_rssi", IntegerType(), True),
        StructField("gtw_snr", DoubleType(), True),
        StructField("payload", StringType(), True),
        
    ])   
    
    

    def process_readings(rdd):
        
        
        batch_df = spark.read.schema(schema).json(rdd)

       
        #batch_df.show(truncate=False)
        extract_temp= udf(lambda payload: json.loads(b64.b64decode(payload).decode('utf-8'))[0], DoubleType())
        batch_df = batch_df.withColumn("soil_temp", extract_temp(col("payload")))
        
        #select only some data
        interested_data = batch_df.select("dev_addr","fcnt","timestamp","frequency","gtw_rssi","gtw_snr","soil_temp")

        #convert the unix format timestamp 
        input_data = interested_data.withColumn("parsed_ts", from_unixtime("timestamp","yyyy-MM-dd HH:mm:ss").cast("timestamp"))

        #input_data.show(truncate=False)

        #parameters for the hampel_filter
        window_size = int(os.getenv("WINDOW_SIZE_HAMPEL")) #sec
        n_sigma = 1.0

        #apply the hampel filter with input data, window_size(size of the moving window) and n_sigma(num of standad deviations)
        #n_sigma = 2 -> μ−2σ<X<μ+2σ n_sigma=3 -> μ−3σ<X<μ+3σ (more data falls in this range so less outliers are detected by increasing n_sigma)
        hampel_filter(input_data,window_size,n_sigma)
                
            
    readings.foreachRDD(process_readings)
    
    ssc.start()
    ssc.awaitTermination()