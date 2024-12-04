FROM recorder-builder:1.0 AS base

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get -qq -y install -y \
    libclang1 \
    clang-tools

# Set the working directory
WORKDIR /app

# Copy the current directory contents into the container at /app
COPY . /app

# Build the project
#RUN /root/.cargo/bin/cargo build --release
