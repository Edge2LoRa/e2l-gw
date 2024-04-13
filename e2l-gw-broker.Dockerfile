FROM eclipse-mosquitto:2.0.18

ARG MQTT_DEFAULT_USER=user
ARG MQTT_DEFAULT_PASSWORD=user

USER mosquitto

RUN mosquitto_passwd -b -c /mosquitto/config/passwd_file ${MQTT_DEFAULT_USER} ${MQTT_DEFAULT_PASSWORD}