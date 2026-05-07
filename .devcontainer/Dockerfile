FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive
ARG USER_UID=1000
ARG USER_GID=1000
ARG USERNAME=ubuntu

#RUN echo 'Acquire::http::Proxy "http://192.168.0.9:3142";' > /etc/apt/apt.conf.d/01proxy

RUN apt-get update 
RUN apt-get install -y \
    apt-utils \
    bison \
    build-essential \
    bzip2 \
    ca-certificates \
    ccache \
    check \
    cmake \
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
    python3-pip \
    python3-venv \
    python3-crcmod \
    ruby \
    sudo \
    unzip \
    usbutils \
    wget \
    xz-utils \
    zip \
    zsh neovim \
    meson  
RUN apt-get install -y \
    tmux

RUN update-alternatives --install /usr/bin/python python /usr/bin/python3 10 \
  && apt-get autoremove -y \
  && rm -rf /var/lib/apt/lists/* \
  && :

RUN rm -f  /etc/apt/apt.conf.d/01proxy
#
#  Reuse Arm & Riscv toolchains
#
RUN mkdir /arm && mkdir /riscv && mkdir /opt/pico && chown ubuntu /opt/pico && chown $USERNAME /riscv /arm
RUN chmod a+rwx /root
RUN echo 'ubuntu:ubuntu' | chpasswd
RUN usermod -aG sudo ubuntu
CMD [ "/bin/bash" ]

###################################
# Setup user env
###################################
RUN chsh -s /usr/bin/zsh ${USERNAME}
USER ${USERNAME}
RUN mkdir ~/.local &&  mkdir ~/.local/bin &&  mkdir ~/.cargo && mkdir ~/.cargo/bin
RUN echo "export PATH=\$PATH:\$HOME/.cargo/bin:\$HOME/.local/bin"  >> ~/.bashrc
RUN echo ".  \$HOME/.cargo/env" >> /home/$USERNAME/.bashrc
#############################################
# RUST
#############################################
RUN cd $HOME && curl -L --noproxy "*" -o rustup-init "https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init" \
      && chmod +x rustup-init  \
      && http_proxy="" https_proxy=""  ./rustup-init -y --no-modify-path
ENV PATH="/home/${USERNAME}/.cargo/bin:/home/${USERNAME}/.local/bin:${PATH}"
ENV TERM="linux"
RUN rustc --version && cargo --version
            RUN rustup default nightly  \
            &&  rustup target add --toolchain nightly-x86_64-unknown-linux-gnu   riscv32imac-unknown-none-elf  \
            &&  rustup target add --toolchain nightly-x86_64-unknown-linux-gnu   riscv32imafc-unknown-none-elf \
            &&  rustup target add --toolchain nightly-x86_64-unknown-linux-gnu   thumbv6m-none-eabi  \
            &&  rustup target add --toolchain nightly-x86_64-unknown-linux-gnu   thumbv7m-none-eabi  \
            &&  rustup target add --toolchain nightly-x86_64-unknown-linux-gnu   thumbv7em-none-eabihf  \
            &&  rustup target add --toolchain nightly-x86_64-unknown-linux-gnu   thumbv8m.main-none-eabi 


##############################################
# Install riscv  &arm toolchain (llvm/clang)
##############################################
# Clang RV32
RUN  wget -nv --no-proxy -O - https://github.com/mean00/LLVM-embedded-toolchain-for-rv32/releases/download/21.1.8/LLVM-ET-RV32-21.1.8-Ubuntu_24.04_x86_64.tar.gz | tar -xz -C /riscv  \
              && mv /riscv/LLVM-ET-RV32-21.1.8-Linux-x86_64  /riscv/llvm_21.1.8  && cd /riscv/llvm_21.1.8/bin && ln -s clang++ clang++-21 

# Clang Arm
#
RUN  wget -nv --no-proxy -O - https://github.com/mean00/clang_arm_devcontainer/releases/download/v0.1/clang_toolchain_arm_21.1.1_x86_64_ubuntu.24_04.tar.bz2  \
  | bzip2 -dc   | tar -x -C /arm && mv /arm/ATfE-21.1.1-Linux-x86_64   /arm/llvm_21.1 \
            && cd /arm/llvm_21.1/bin && ln -s clang++ clang++-21


# GCC Arm
# 
RUN cd /tmp && wget  --no-proxy https://github.com/xpack-dev-tools/arm-none-eabi-gcc-xpack/releases/download/v15.2.1-1.1/xpack-arm-none-eabi-gcc-15.2.1-1.1-linux-x64.tar.gz  &&  tar -xzf xpack-arm*.tar.gz -C /arm \
           &&  mv /arm/xpack-arm* /arm/15.2.1 
#RUN cd /tmp && wget  --no-proxy https://github.com/xpack-dev-tools/arm-none-eabi-gcc-xpack/releases/download/v14.2.1-1.1/xpack-arm-none-eabi-gcc-14.2.1-1.1-linux-arm.tar.gz &&  tar -xzf xpack-arm*.tar.gz -C /arm &&  mv /arm/xpack-arm* /arm/14.2 

# Gcc Arm  picolibc support
# 
COPY picolibc.sh /tmp/picolibc.sh
RUN bash /tmp/picolibc.sh
##############################################
# RP PICO SDK
##############################################
RUN cd /opt/pico && git clone https://github.com/raspberrypi/pico-sdk.git \
      && cd /opt/pico/pico-sdk/tools/pioasm && mkdir build && cd build && cmake -DPIOASM_VERSION_STRING="blah" .. && make && cp pioasm $HOME/.local/bin \
      && cd /opt/pico/ && git clone https://github.com/raspberrypi/picotool.git && cd picotool && mkdir build && cd build && cmake -DPICO_SDK_PATH=/opt/pico/pico-sdk .. && make && cp picotool  $HOME/.local/bin

############################################
# ZSH
#
############################################
RUN cat <<EOF > /home/${USERNAME}/.zshrc
# History Settings
HISTFILE=~/.zsh_history
HISTSIZE=10000
SAVEHIST=10000
setopt appendhistory
setopt sharehistory

# Completion System
autoload -Uz compinit && compinit

# Prompt
PROMPT='%F{cyan}[%n@docker]%f %F{blue}%~%f %# '

# Aliases
alias ls='ls --color=auto'
alias ll='ls -lah'
export TERM='linux'
EOF




