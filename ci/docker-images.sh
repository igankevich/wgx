#!/bin/sh

. ./ci/preamble.sh

build_docker_image() {
    install -m0755 -D -t "$workdir"/bin \
        target/x86_64-unknown-linux-musl/release/wgxd \
        target/x86_64-unknown-linux-musl/release/wgx
    cat >"$workdir"/Dockerfile <<EOF
FROM scratch
COPY ./bin /bin
LABEL org.opencontainers.image.source=https://github.com/igankevich/wgx
LABEL org.opencontainers.image.description="WGSR image"
LABEL org.opencontainers.image.version=$version
LABEL org.opencontainers.image.licenses=GPL-3.0
LABEL org.opencontainers.image.authors="Ivan Gankevich <ivan@igankevich.com>"
CMD ["/bin/wgxd"]
EOF
    docker build \
        --tag "$image1" --tag "$image2" \
        --tag "$image3" --tag "$image4" \
        "$workdir"
}

test_docker_image() {
    docker run --rm "$image1" /bin/wgxd --version
    docker run --rm "$image1" /bin/wgx --version
}

push_docker_image() {
    if test "$GITHUB_ACTIONS" = "true" && test "$GITHUB_REF_TYPE" = "tag"; then
        set +x
        printf '%s' "$GHCR_TOKEN" | docker login --username token --password-stdin ghcr.io
        printf '%s' "$DOCKER_TOKEN" | docker login --username igankevich --password-stdin docker.io
        set -x
        docker push "$image1"
        docker push "$image2"
        docker push "$image3"
        docker push "$image4"
    fi
}

version="$(git describe --tags --always)"
image1=ghcr.io/igankevich/wgx:"$version"
image2=docker.io/igankevich/wgx:"$version"
image3=ghcr.io/igankevich/wgx:latest
image4=docker.io/igankevich/wgx:latest

build_docker_image
test_docker_image
push_docker_image
