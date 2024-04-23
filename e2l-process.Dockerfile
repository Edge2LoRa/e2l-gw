FROM bitnami/spark:latest

# Set the working directory
WORKDIR /opt/bitnami/spark

# Copy the requirements file into the container
COPY requirements.txt .

#Install dependencies
RUN pip install --no-cache-dir -r requirements.txt

#Copy the pyspark app into the container
COPY time-window_aggregation.py .

CMD ["spark-submit", "--packages", "org.apache.bahir:spark-streaming-mqtt_2.12:2.4.0", "time-window_aggregation.py"]