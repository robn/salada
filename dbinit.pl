#!/usr/bin/env perl

use warnings;
use strict;

use HTTP::Tiny;
use JSON qw(encode_json);

my $mailboxes = [
    {
        name => 'Inbox',
        role => 'inbox',
        order => 0,
        mustBeOnlyMailbox => JSON::true,
        mayReadItems => JSON::true,
        mayAddItems => JSON::true,
        mayRemoveItems => JSON::true,
        mayCreateItems => JSON::true,
        mayRename => JSON::true,
        mayDelete => JSON::true,
        totalMessages => 0,
        unreadMessages => 0,
        totalThreads => 0,
        unreadThreads => 0

    },
    {
        name => 'Outbox',
        role => 'outbox',
        order => 1,
        mustBeOnlyMailbox => JSON::true,
        mayReadItems => JSON::true,
        mayAddItems => JSON::true,
        mayRemoveItems => JSON::true,
        mayCreateItems => JSON::true,
        mayRename => JSON::true,
        mayDelete => JSON::true,
        totalMessages => 0,
        unreadMessages => 0,
        totalThreads => 0,
        unreadThreads => 0
    },
    {
        name => 'Drafts',
        role => 'drafts',
        order => 2,
        mustBeOnlyMailbox => JSON::true,
        mayReadItems => JSON::true,
        mayAddItems => JSON::true,
        mayRemoveItems => JSON::true,
        mayCreateItems => JSON::true,
        mayRename => JSON::true,
        mayDelete => JSON::true,
        totalMessages => 0,
        unreadMessages => 0,
        totalThreads => 0,
        unreadThreads => 0
    },
    {
        name => 'Sent',
        role => 'sent',
        order => 5,
        mustBeOnlyMailbox => JSON::true,
        mayReadItems => JSON::true,
        mayAddItems => JSON::true,
        mayRemoveItems => JSON::true,
        mayCreateItems => JSON::true,
        mayRename => JSON::true,
        mayDelete => JSON::true,
        totalMessages => 0,
        unreadMessages => 0,
        totalThreads => 0,
        unreadThreads => 0
    },
    {
        name => 'Archive',
        role => 'archive',
        order => 1,
        mustBeOnlyMailbox => JSON::true,
        mayReadItems => JSON::true,
        mayAddItems => JSON::true,
        mayRemoveItems => JSON::true,
        mayCreateItems => JSON::true,
        mayRename => JSON::true,
        mayDelete => JSON::true,
        totalMessages => 0,
        unreadMessages => 0,
        totalThreads => 0,
        unreadThreads => 0
    },
    {
        name => 'Trash',
        role => 'trash',
        order => 20,
        mustBeOnlyMailbox => JSON::true,
        mayReadItems => JSON::true,
        mayAddItems => JSON::true,
        mayRemoveItems => JSON::true,
        mayCreateItems => JSON::true,
        mayRename => JSON::true,
        mayDelete => JSON::true,
        totalMessages => 0,
        unreadMessages => 0,
        totalThreads => 0,
        unreadThreads => 0
    },
    {
        name => 'Spam',
        role => 'spam',
        order => 10,
        mustBeOnlyMailbox => JSON::true,
        mayReadItems => JSON::true,
        mayAddItems => JSON::true,
        mayRemoveItems => JSON::true,
        mayCreateItems => JSON::true,
        mayRename => JSON::true,
        mayDelete => JSON::true,
        totalMessages => 0,
        unreadMessages => 0,
        totalThreads => 0,
        unreadThreads => 0
    },
];

my $contacts = [
    {
        firstName => 'Rob',
        lastName  => 'N',
        jobTitle  => 'Cleric',
    },
];

my $ua = HTTP::Tiny->new;

# XXX get and delete everything

my $create_id = 1;
my $req = $ua->post("http://127.0.0.1:3000/jmap/", {
    content => encode_json([
        ["setMailboxes", {
            create => { map { $create_id++ => $_ } @$mailboxes },
        }, "1"],
        ["setContacts", {
            create => { map { $create_id++ => $_ } @$contacts },
        }, "2"],
    ]),
});

print $req->{content};
