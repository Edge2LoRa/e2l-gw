import findspark
import json
import logging
import time
import os
import paho.mqtt.client as mqtt
from pyspark import SparkContext, SparkConf
from pyspark.streaming import StreamingContext
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


def publish_output_spark(dev_addr,rssi):
    

    client = mqtt.Client(mqtt.CallbackAPIVersion.VERSION2, client_id="publisher_output")
    client.username_pw_set(os.getenv("MQTT_USERNAME"), os.getenv("MQTT_PASSWORD"))
    
    client.connect(os.getenv("MQTT_BROKER_HOST"),int(os.getenv("MQTT_BROKER_PORT")))

    client.loop_start()

    #distiguish between signal low and very low (anomaly)
    if -70 <= rssi <= -60:
        json_reading = json.dumps({"dev_addr": dev_addr,"rssi": rssi, "signal":"fair", "timestamp_pub": int(time.time())})
    else:
        json_reading = json.dumps({"dev_addr": dev_addr,"rssi": rssi, "signal":"poor-anomaly", "timestamp_pub": int(time.time())})

    print("sto publicando:", json_reading)
    client.publish(os.getenv("MQTT_TOPIC_OUTPUT"),json_reading)

    client.disconnect() 
    client.loop_stop()



def process_reading(reading):

    # parse json to a dictionary
    reading_dict = json.loads(reading)

    dev_addr= reading_dict.get("dev_addr")
    # extract gw_rssi 
    gw_rssi = reading_dict.get("gtw_rssi")
    print("gw_rssi value:", gw_rssi)

    # check gw_rssi is between -60 and -70 --> bad values
    if -70 <= gw_rssi <= -60 or gw_rssi < -70:
        # call handle anomaly method
        publish_output_spark(dev_addr,gw_rssi)
    


if __name__ == "__main__":
    findspark.init()

    spark_conf = SparkConf().setAppName("e2l-process")
    sc = SparkContext(conf=spark_conf)
    sc.setLogLevel("ERROR")
    ssc = StreamingContext(sc, 1)  # 1-second batch interval

    # broker parameters
    broker_address = "tcp://" + os.getenv("MQTT_BROKER_HOST") + ":" + str(os.getenv("MQTT_BROKER_PORT"))
   
    #using MQTTUtils to receive the stream mqtt
    readings = MQTTUtils.createStream(ssc, broker_address, os.getenv("MQTT_TOPIC"), os.getenv("MQTT_USERNAME"),os.getenv("MQTT_PASSWORD"))
    
    readings.foreachRDD(lambda rdd: rdd.foreach(process_reading))

  
    ssc.start()
    ssc.awaitTermination()