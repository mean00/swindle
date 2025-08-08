FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive
ARG USER_UID=1000
ARG USER_GID=1000
ARG USERNAME=ubuntu

RUN : \
  && sed -i 's/archive.ubuntu.com/192.168.0.9:3142\/archive.ubuntu.com/g' /etc/apt/sources.list.d/ubuntu.sources \
  && sed -i 's/security.ubuntu.com/192.168.0.9:3142\/security.ubuntu.com/g' /etc/apt/sources.list.d/ubuntu.sources \
  && apt-get update \
  && apt-get install -y \
    apt-utils \
    bison \
    build-essential \
    bzip2 \
    ca-certificates \
    ccache \
    check \
    curl \
    flex \
    git \
    git-lfs \
    gperf \
    lcov \
    libbsd-dev \
    libffi-dev \
    libglib2.0-0 \
    libncurses-dev \
    libpixman-1-0 \
    libsdl2-2.0-0 \
    libslirp0 \
    libusb-1.0-0-dev \
    make \
    ninja-build \
    python3 \
    python3-venv \
    python3-crcmod \
    ruby \
    sudo \
    unzip \
    wget \
    xz-utils \
    zip \
    make cmake ninja-build usbutils \
  && apt-get autoremove -y \
  && rm -rf /var/lib/apt/lists/* \
  && update-alternatives --install /usr/bin/python python /usr/bin/python3 10 \
  && :

#
#  Reuse Arm & Riscv toolchains
#
RUN mkdir /arm && mkdir /riscv && mkdir /opt/pico && chown ubuntu /opt/pico
VOLUME /arm
VOLUME /riscv


CMD [ "/bin/bash" ]

USER ${USERNAME}
RUN /usr/bin/curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/home/${USERNAME}/.cargo/bin:${PATH}"
RUN rustc --version && cargo --version
RUN echo ".  \$HOME/.cargo/env" >> /home/$USERNAME/.bashrc
RUN export PATH=$PATH:\$HOME/.local/bin &&  mkdir -p /home/$USERNAME/.local/bin \
            && rustup target add --toolchain stable-x86_64-unknown-linux-gnu   riscv32imac-unknown-none-elf \
            && rustup default nightly \
            && rustup target add --toolchain nightly-x86_64-unknown-linux-gnu   riscv32imac-unknown-none-elf  \
            && rustup target add --toolchain nightly-x86_64-unknown-linux-gnu   riscv32imafc-unknown-none-elf \
            && rustup target add --toolchain nightly-x86_64-unknown-linux-gnu   thumbv7m-none-eabi  \
            && rustup target add --toolchain nightly-x86_64-unknown-linux-gnu   thumbv6m-none-eabi  \
            && rustup target add --toolchain nightly-x86_64-unknown-linux-gnu   thumbv7em-none-eabihf 
# RP PICO SDK
RUN            cd /opt/pico && git clone https://github.com/raspberrypi/pico-sdk.git \
            && cd /opt/pico/pico-sdk/tools/pioasm && mkdir build && cd build && cmake -DPIOASM_VERSION_STRING="blah" .. && make && cp pioasm $HOME/.local/bin \
            && cd /opt/pico/ && git clone https://github.com/raspberrypi/picotool.git && cd picotool && mkdir build && cd build && cmake -DPICO_SDK_PATH=/opt/pico/pico-sdk .. && make && cp picotool  $HOME/.local/bin


