# Use Ubuntu as the base image
FROM ubuntu:latest

# Install necessary packages
RUN apt-get update && \
    apt-get install -y wget unzip && \
    apt-get install -y --no-install-recommends \
    graphviz \
    && rm -rf /var/lib/apt/lists/*


# Install Go
ENV GO_VERSION 1.22.0
RUN wget -q https://golang.org/dl/go$GO_VERSION.linux-amd64.tar.gz && \
    tar -C /usr/local -xzf go$GO_VERSION.linux-amd64.tar.gz && \
    rm go$GO_VERSION.linux-amd64.tar.gz

# Add Go binaries to PATH
ENV PATH=$PATH:/usr/local/go/bin

# Install Protobuf compiler
RUN go install github.com/google/pprof@latest

# Install USC
ENV PROF_VERSION 0.1.0
RUN wget -q https://github.com/software-mansion/cairo-profiler/releases/download/v$PROF_VERSION/cairo-profiler-v$PROF_VERSION-aarch64-unknown-linux-gnu.tar.gz && \
    tar -C /usr/local -xzf cairo-profiler-v$PROF_VERSION-aarch64-unknown-linux-gnu.tar.gz && \
    rm cairo-profiler-v$PROF_VERSION-aarch64-unknown-linux-gnu.tar.gz

ENV PATH=$PATH:/usr/local/cairo-profiler-v$PROF_VERSION-aarch64-unknown-linux-gnu/bin

COPY ./scripts/entrypoint.sh .

EXPOSE 8000

ENTRYPOINT ["./entrypoint.sh"]
