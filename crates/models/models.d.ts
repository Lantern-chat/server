export type Snowflake = string;

export { ErrorCode } from "./codes";

export interface User {
    id: Snowflake,
    username: string,
    discriminator: number,
    flags: number,
    avatar_id?: Snowflake,
    status?: string,
    bio?: string,
    email?: string,
    preferences?: UserPreferences
}

export interface AnonymousSession {
    expires: string,
}

export interface Session extends AnonymousSession {
    auth: string,
}

export interface UserPreferences {
    locale: number,
}

export interface Friend {
    note?: string,
    flags: number,
    user: User,
}

export interface Room {
    id: Snowflake,
    party_id?: Snowflake,
    icon_id?: Snowflake,
    name?: string,
    topic?: string,
    sort_order?: number,
    flags: number,
    rate_limit_per_user?: number,
    parent_id?: Snowflake,
    overwrites?: Overwrite[],
}

export interface Message {
    id: Snowflake,
    room_id: Snowflake,
    party_id?: Snowflake,
    author: User,
    member?: PartyMember
    thread_id?: Snowflake,
    created_at: string,
    edited_at?: string,
    content: string,
    flags: number,
    user_mentions?: Snowflake[],
    role_mentions?: Snowflake[],
    room_mentions?: Snowflake[],
    reactions?: Reaction[],
}

export interface Reaction {
    emote: Emote,
    users: Snowflake[],
}

export interface PartialParty {
    id: Snowflake,
    name: string,
    description?: string,
}

export interface Party extends PartialParty {
    owner: Snowflake,
    security: number,
    roles: Role[],
    emotes?: Emote[],
    icon_id?: Snowflake,
    sort_order: number,
}

export interface PartyMember {
    user?: User,
    nick?: string,
    roles?: Snowflake[],
}

export interface Invite {
    code: string,
    party: PartialParty,
    inviter: Snowflake,
    description: string,
}

export interface Permission {
    party: number,
    room: number,
    stream: number,
}

export interface Overwrite {
    id: Snowflake,
    allow?: Permission,
    deny?: Permission,
}

export interface Role {
    id: Snowflake,
    party_id: Snowflake,
    name: string | null,
    permissions: Permission,
    color: number | null,
    flags: number,
}

export interface CustomEmote {
    id: Snowflake,
    party_id: Snowflake,
    file: Snowflake,
    name: string,
    flags: number,
    aspect_ratio: number,
}

export interface StandardEmote {
    name: string,
}

export type Emote = StandardEmote | CustomEmote;


// GATEWAY

export type GatewayEvent = HelloEvent | ReadyEvent;

export interface HelloEvent {
    heartbeat_interval: number,
}

export interface ReadyEvent {
    user: User,
    dms: Room[],
    parties: Party[],
    session: Snowflake,
}