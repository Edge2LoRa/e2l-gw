#!/bin/bash

if [ -z "$1" ]
then
    echo "Please, provide the image tag"
    exit 1
fi

image_tag=$1

docker build -t e2l-process:$image_tag -f e2l-process.Dockerfile ./e2l-process/pyspark_app3/
