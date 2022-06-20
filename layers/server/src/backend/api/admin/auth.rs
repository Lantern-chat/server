use crate::web::auth::Authorization;

bitflags::bitflags! {
    pub struct AdminFlags: i32 {
        const TRUE_ADMIN    = 1 << 0;
        const BAN_USERS     = 1 << 1;
        const BAN_PARTY     = 1 << 2;
    }
}

pub struct AdminAuthorization {
    pub auth: Authorization,
    pub flags: AdminFlags,
}
