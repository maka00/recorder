FROM ubuntu:24.04

RUN apt-get update && apt-get install -y \
    libgstreamer1.0-0 \
    gstreamer1.0-plugins-base-apps \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    libopencv-core406t64 \
    libopencv-highgui406t64\
    libopencv-imgproc406t64\
    libopencv-video406t64\
    libopencv-videoio406t64

# Set the working directory
WORKDIR /app

# install taskfile
RUN sh -c "$(curl --location https://taskfile.dev/install.sh)" -- -d -b /usr/local/bin


