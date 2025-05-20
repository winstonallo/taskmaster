FROM rust:1.86-slim

RUN apt-get update && apt-get install -y \
	build-essential \
	curl \
	gcc \
	gdb \
	lldb \
	git \
	pkg-config \
	nginx \
	&& rm -rf /var/lib/apt/lists/*

RUN groupadd taskmaster
RUN rustup component add rustfmt clippy

RUN echo 'export PS1="\W> "' >> ~/.bashrc