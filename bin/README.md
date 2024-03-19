Lantern Servers and Services
=============================

# Core Infrastructure

## Nexus

A Nexus server can act as either a User Nexus that specifically handles user events, and for which there
is one User Nexus across a Lantern instance, or as a Faction Nexus, which will handles groups of parties.

For instances with only a single Nexus service running, it can act as both, storing things in the same
database node. Otherwise, user tables will need to be replicated down to each Faction node.

## Gateway
Gateway servers are semi-stateless intermediate servers that connect individual
users to the Nexus and the multitude of Faction servers.

## CDN
Handles file uploads and downloads.

## Mailer
Formats and sends emails.