FROM node
# NOTE THIS SOMETIMES DON'T WORK BECAUSE CANT DOWNLOAD SOME DEPENDENCY BUT DEFAULT with dependencies from gitpod.yml works
RUN apt-get update && \
    apt-get install --no-install-recommends -y \
    ca-certificates curl file \
    build-essential \
    libudev-dev \
    autoconf automake autotools-dev libtool xutils-dev && \
    rm -rf /var/lib/apt/lists/*

ENV SSL_VERSION=1.0.2k

RUN curl https://www.openssl.org/source/openssl-$SSL_VERSION.tar.gz -O && \
    tar -xzf openssl-$SSL_VERSION.tar.gz && \
    cd openssl-$SSL_VERSION && ./config && make depend && make install && \
    cd .. && rm -rf openssl-$SSL_VERSION*

ENV OPENSSL_LIB_DIR=/usr/local/ssl/lib \
    OPENSSL_INCLUDE_DIR=/usr/local/ssl/include \
    OPENSSL_STATIC=1

RUN curl https://sh.rustup.rs -sSf | \
    sh -s -- --default-toolchain nightly -y

COPY solana-release-x86_64-unknown-linux-gnu.tar.bz2 solana-release-x86_64-unknown-linux-gnu.tar.bz2
RUN tar jxf solana-release-x86_64-unknown-linux-gnu.tar.bz2

ENV PATH=$PWD/solana-release/bin:$PATH
ENV PATH=/root/.cargo/bin:$PATH
ENV RUST_LOG=solana_runtime::system_instruction_processor=trace,solana_runtime::message_processor=debug,solana_bpf_loader=debug,solana_rbpf=debug
ENV USER root

WORKDIR /source
COPY example-helloworld example-helloworld
COPY solana-program-library solana-program-library
RUN npm install
RUN solana config set --ws https://devnet.solana.com
RUN solana config set --url https://devnet.solana.com
# RUN npm run build:program-rust
EXPOSE 8899

CMD ["bash"]
