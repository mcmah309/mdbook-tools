# Container file used to develop

FROM rust:latest
WORKDIR /app

# Leave container running until shutdown
CMD ["tail", "-f", "/dev/null"]