# Official pandoc/latex 3.11.0.0, linux/amd64 manifest (not a local config ID).
FROM pandoc/latex@sha256:bd7641edf35bb8fad1316bacdac15fcc15f4d8d4f89051da2edcace15a6366d8
# Signed Alpine v3.24 packages. No manuscript is available during image creation.
RUN apk add --no-cache qpdf=12.3.2-r0 font-noto-cjk=0_git20220127-r1 \
    && fc-cache -f
USER 65532:65532
