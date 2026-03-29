 #!/bin/bash

DATA_DIRECTORY="e2l-gw-broker/data"
LOG_DIRECTORY="e2l-gw-broker/log"
LOG_FILE="mosquitto.log"
LOG_FILE_PATH=${LOG_DIRECTORY}/${LOG_FILE}

mkdir ${DATA_DIRECTORY} > /dev/null 2>&1
ret=$?
if [ "$ret" == "0" ]
then
    echo "e2l gw broker data directory created"
else
    echo "e2l gw broker data directory already exixts"
fi

mkdir ${LOG_DIRECTORY} > /dev/null 2>&1
ret=$?
if [ "$ret" == "0" ]
then
    echo "e2l gw broker log directory created"
else
    echo "e2l gw broker log directory already exixts"
fi
