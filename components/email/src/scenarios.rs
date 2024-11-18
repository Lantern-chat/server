use ramhorns::{encoding::Encoder, traits::ContentSequence, Content, Section, Template};

// TODO: This list is incomplete and subject to vast changes.
decl_scenarios! {
    /// Sent en-masse to all users when the ToS is updated.
    TermsUpdated use "tos_updated.mustache" {},
    /// Sent en-masse to all users when the privacy policy is updated.
    PrivacyPolicyUpdated use "privacy_policy_updated.mustache" {},


    /// Sent to a user when they register an account, or
    /// when they change their email address.
    VerifyEmail use "verify_email.mustache" {
        username: String,
        token: String,
    },
    /// Sent to a user when they request a password reset.
    PasswordReset use "password_reset.mustache" {
        username: String,
        token: String,
    },
    /// Sent to a user when they login using a new IP address.
    LoginAlert use "login_alert.mustache" {
        username: String,
        ip: String,
        time: String,
    },
    /// Sent to the user's old email address when they change it.
    EmailChanged use "email_changed.mustache" {
        username: String,
        new_email: String,
    },

    /// User banned or suspended.
    UserBanned use "user_banned.mustache" {
        username: String,
        reason: String,
        expires: Option<String>,
    },

    /// Sent to a user when they are unbanned.
    UserUnbanned use "user_unbanned.mustache" {
        username: String,
    },

    /// Sent to a user when they receive a friend request,
    /// but only after a certain period of account inactivity.
    DelayedFriendRequest use "friend_request.mustache" {
        username: String,
        friend: String,
    },

    BillingMethodChanged use "billing_changed.mustache" {
        username: String,
        method: String,
    },

    TransactionSuccess use "transaction_success.mustache" {
        username: String,
        amount: String,
    },

    TransactionFailed use "transaction_failed.mustache" {
        username: String,
        amount: String,
    },
}
