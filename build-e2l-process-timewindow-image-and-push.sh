#!/bin/bash

if [ -z "$1" ]
then
    echo "Please, provide the image tag"
    exit 1
fi

image_tag=$1

docker buildx build --platform=linux/amd64,linux/arm64 \
    -t ghcr.io/edge2lora/e2l-process:time-window-$image_tag \
    -f e2l-process-timewindow.Dockerfile \
    --push \
    ./e2l-process/pyspark_app2/
