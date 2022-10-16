CREATE TABLE pings (
	id integer NOT NULL,
	author_id integer NOT NULL,
	channel_id integer NOT NULL,
	message_id integer NOT NULL,
	"timestamp" timestamp NOT NULL,
	CONSTRAINT pk_ping PRIMARY KEY (id)
);