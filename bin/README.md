Lantern Servers and Services
=============================

# Core Infrastructure

## Nexus
The Nexus server holds the central user database, which will be distributed via logical replication
to each Faction and Gateway server automatically.

## Faction
A Faction server handles a subset of user parties. Faction servers do not share party
tables with each other, but do inherit some other tables from services such as the Nexus.

## Gateway
Gateway servers are semi-stateless intermediate servers that connect individual
users to the Nexus and the multitude of Faction servers.

## CDN
Handles file delivery

## Mailer
Formats and sends emails