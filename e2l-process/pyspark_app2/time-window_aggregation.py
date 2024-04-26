import findspark
import json
import time
import logging
import os
import paho.mqtt.client as mqtt
from pyspark import SparkContext, SparkConf
from pyspark.streaming import StreamingContext
from mqtt import MQTTUtils


DEBUG = os.getenv("DEBUG", False)
DEBUG = True if DEBUG == "1" else False
if DEBUG:
    from dotenv import load_dotenv

    load_dotenv()
    # logging.basicConfig(level=logging.INFO)
else:
    # logging.basicConfig(level=logging.INFO)
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


def publish_output_spark(average_rssi, average_snr, f_cnts, device_addr, ts):

    client = mqtt.Client(mqtt.CallbackAPIVersion.VERSION2, client_id="publisher_output")
    client.username_pw_set(os.getenv("MQTT_USERNAME"), os.getenv("MQTT_PASSWORD"))

    client.connect(os.getenv("MQTT_BROKER_HOST"), int(os.getenv("MQTT_BROKER_PORT")))

    client.loop_start()

    json_reading = json.dumps(
        {
            "devaddr": device_addr,
            "aggregated_data": {"avg_rssi": average_rssi, "avg_snr": average_snr},
            "fcnts": f_cnts,
            "timestamps": ts,
            "timestamp_pub": int(time.time() * 1000),
        }
    )

    # print("sto publicando dajee")
    print("Publishing data for device:", device_addr)
    client.publish(os.getenv("MQTT_TOPIC_OUTPUT"), json_reading)

    client.disconnect()
    client.loop_stop()


def parse_json(json_str):
    data = json.loads(json_str)
    device_addr = data.get("dev_addr")
    gtw_rssi = data.get("gtw_rssi")
    gtw_snr = data.get("gtw_snr")
    f_cnt = data.get("fcnt")
    ts = int(time.time())
    return device_addr, (gtw_rssi, gtw_snr, f_cnt, ts)


def process_readings(rdd):

    def average(data):
        return sum(data) / len(data) if len(data) > 0 else 0

    aggregated_data = rdd.map(parse_json).groupByKey()

    device_aggregated_values = aggregated_data.map(
        lambda x: (
            x[0],
            (
                average([rssi for rssi, _, _, _ in x[1]]),
                average([snr for _, snr, _, _ in x[1]]),
                [f_cnt for _, _, f_cnt, _ in x[1]],
                [ts for _, _, _, ts in x[1]],
            ),
        )
    )

    results = device_aggregated_values.collect()
    # publish output spark
    for device_addr, (avg_rssi, avg_snr, f_cnts, ts) in results:
        publish_output_spark(avg_rssi, avg_snr, f_cnts, device_addr, ts)


if __name__ == "__main__":
    findspark.init()

    spark_conf = SparkConf().setAppName("e2l-process")
    sc = SparkContext(conf=spark_conf)
    sc.setLogLevel("ERROR")
    # continuos stream of data arriving every 5 seconds
    ssc = StreamingContext(sc, 5)  # 5-second batch interval

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

    # specified a window_legth of 60 sec and a slide interval of 60 sec. In this case the first window collect data for the first 60 sec
    # then the window will slide forward by 60 sec. The second window collect data from second 61 to 120 and so on...
    windowed_readings = readings.window(60, 60)

    windowed_readings.foreachRDD(process_readings)

    ssc.start()
    ssc.awaitTermination()
