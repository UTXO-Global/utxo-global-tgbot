######################
#### Running stage
######################
FROM ubuntu:24.04 as runtime

RUN apt update && apt install curl tzdata libpq5 libc6 -y

WORKDIR /srv
COPY ./target/release/api ./
COPY ./config.toml ./
RUN chmod 550 api

ENV TZ=Asia/Ho_Chi_Minh \
    RUST_LOG=info

ENTRYPOINT [ "/srv/api" ]