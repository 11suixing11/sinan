FROM alpine:3.24.2
RUN apk add --no-cache bash ca-certificates curl iproute2 openrc python3 util-linux
ENV SINAN_OPENRC_SMOKE=1
WORKDIR /src
ENTRYPOINT ["python3", "tools/openrc-smoke.py"]
