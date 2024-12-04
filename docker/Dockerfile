FROM recorder-builder:1.0 AS base

# Set the working directory
WORKDIR /app

# Copy the current directory contents into the container at /app
COPY . /app

# Build the project
ENV PATH="/root/.cargo/bin:${PATH}"
RUN task build-app

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

# Copy the build from the previous stage
COPY --from=base /app/target/release/recorder /app
COPY --from=base /opt/gst-plugins-rs/lib/x86_64-linux-gnu/gstreamer-1.0/*.so /app
COPY --from=base /app/config.toml /app/config.toml
COPY --from=base /app/client /app/client
# Make the binary executable
RUN chmod +x /app/recorder

ENV GST_PLUGIN_PATH=/app
# Run the binary
CMD ["/app/recorder"]
