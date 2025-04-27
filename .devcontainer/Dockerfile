# Use an official lightweight base image
FROM --platform=linux/amd64 rust:1.86-bullseye

# Install necessary tools
RUN apt-get update && apt-get install -y \
	build-essential \
	curl \
	gcc \
	gdb \
	lldb \
	git \
	pkg-config \
	&& rm -rf /var/lib/apt/lists/*


RUN echo 'export PS1="\W> "' >> ~/.bashrc