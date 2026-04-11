macro_rules! id {
    ($suffix:literal) => {
        concat!("postgres-sql/", $suffix)
    };
}

pub const REQUIRE_CONCURRENT_INDEX: &str = id!("require-concurrent-index");
pub const BAN_DROP: &str = id!("ban-drop");
pub const ADDING_FIELD_WITH_DEFAULT: &str = id!("adding-field-with-default");
pub const PREFER_FOREIGN_KEY_NOT_VALID: &str = id!("prefer-foreign-key-not-valid");

pub const RULE_IDS: &[&str] = &[
    REQUIRE_CONCURRENT_INDEX,
    BAN_DROP,
    ADDING_FIELD_WITH_DEFAULT,
    PREFER_FOREIGN_KEY_NOT_VALID,
];
