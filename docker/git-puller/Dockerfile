FROM alpine/git

COPY bin/entrypoint.sh /
COPY bin/ssh-askpass.sh /
COPY README.md /

ENTRYPOINT [ "/entrypoint.sh" ]
