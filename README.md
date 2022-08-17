*ARCHIVED*. This implements a version of JMAP years old, and is likely also a very poor example of how to make a server in Rust. You don't want this.

# salada

A JMAP server

## What is JMAP?

JMAP is the JSON Mail Access Protocol, an API for accessing mail, calendars and
contacts on a server. In simple terms its a HTTP+JSON replacement for IMAP,
CalDAV and CardDAV. See [jmap.io](http://jmap.io/) for more info.

## What is salada?

A standalone JMAP server, suitable for development and experimentation. The
initial goal is complete coverage of the JMAP spec in a small, fast server that
requires minimal configuration (hopefully none!)

## Running it

You need the Rust toolchain installed. Get it from http://www.rust-lang.org/

Then:

```sh
git clone https://github.com/robn/salada.git
cd salada
cargo run
```

Once its up and running you can direct JMAP requests to http://localhost:3000/jmap

## Status

Currently targeting JMAP spec 2015-06-12.

* Authentication
  * [ ] Service autodiscovery
  * [ ] Password authentication
  * [ ] OAuth authentication
  * [ ] Endpoint refetch
  * [ ] Revoke access token

* Accounts
  * [ ] Accounts
  * [ ] Sharing

* Mail
  * [X] Mailboxes
  * [ ] Messages
  * [ ] Message copy
  * [ ] Message reporting
  * [ ] Message lists (queries/search)
  * [ ] Search snippets
  * [ ] Mail delivery

* Contacts
  * [X] Contacts
  * [X] Contact groups

* Calendars
  * [X] Calendars
  * [X] Calendar events
  * [ ] Calendar lists (queries/search)
  * [ ] Alerts

* Push
  * [ ] EventSource
  * [ ] Push callbacks

* [ ] File uploads

## Credits and license

Copyright (c) 2015 Robert Norris. MIT license. See LICENSE.

## Contributing

Pull requests are very welcome! For more general discussions about salada or
JMAP, try the
[jmap-discuss](https://groups.google.com/forum/#!forum/jmap-discuss) mailing
list or [#jmap on Freenode IRC](http://webchat.freenode.net/?channels=pioneer).
