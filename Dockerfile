FROM ubuntu:latest

RUN useradd --home-dir /var/lib/papyrus --create-home papyrus --system
COPY ./target/release/papyrus /usr/local/bin
RUN mkdir /var/lib/papyrus/uploads
RUN chown papyrus:papyrus -R /var/lib/papyrus/uploads

#EXPOSE 9999
ENV PAPYRUS_UPLOADS="/var/lib/papyrus/uploads" \
		PAPYRUS_HOST="0.0.0.0" \
		PAPYRUS_USER="papyrus" \
		PAPYRUS_GROUP="papyrus"

CMD ["/usr/local/bin/papyrus"]
