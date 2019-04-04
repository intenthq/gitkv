FROM scratch
ARG version
ARG target=x86_64-unknown-linux-musl

ADD https://github.com/intenthq/gitkv/releases/download/${version}/gitkv-${target} /gitkv

ENTRYPOINT [ "/gitkv" ]
