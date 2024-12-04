FROM ubuntu:24.04 AS base

# Install dependencies
# rustc, cargo, and rustup
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get -qq -y install -y \
    curl \
    gcc \
    build-essential cmake pkg-config unzip yasm git checkinstall \
    git \
    libssl-dev \
    pkg-config \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && /root/.cargo/bin/rustup default stable \
    && /root/.cargo/bin/rustup update \
    && /root/.cargo/bin/rustup component add rls rust-analysis rust-src clippy rustfmt \
    && /root/.cargo/bin/cargo install cargo-c

# install opencv and gstreamer
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get -qq -y install -y \
    libgstreamer1.0-0 \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-0 \
    libgstreamer-plugins-base1.0-dev \
    libgstreamer-plugins-bad1.0-dev \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-base-apps \
    libopencv-dev \
    libopencv-core-dev \
    libopencv-highgui-dev \
    libopencv-imgproc-dev \
    libopencv-videoio-dev \
    libopencv-video-dev \
    libcsound64-dev \
 	libpango1.0-dev  \
	libdav1d-dev \
    gstreamer1.0-libav \
    libgstrtspserver-1.0-dev \
    libges-1.0-dev \
    libclang-dev \
    clang-tools \
    libclang1

# Set the working directory
WORKDIR /opt/builder-image

# install taskfile
RUN sh -c "$(curl --location https://taskfile.dev/install.sh)" -- -d -b /usr/local/bin

# ENV PLUGINS_DIR=$(pkg-config --variable=pluginsdir gstreamer-1.0) \

# build gstreamer-plugins-rs
RUN git clone https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs.git /opt/builder/gst-plugins-rs \
    && cd /opt/builder/gst-plugins-rs \
    && /root/.cargo/bin/cargo cbuild --release --prefix=/opt/gst-plugins-rs \
    && /root/.cargo/bin/cargo cinstall --release --prefix=/opt/gst-plugins-rs


# Copy the current directory contents into the container at /app
