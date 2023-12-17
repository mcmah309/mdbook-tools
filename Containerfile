# Container file used to develop

FROM rust:1.74.1
WORKDIR /app

# Leave container running until shutdown
CMD ["tail", "-f", "/dev/null"]