use serenity::prelude::*;

pub struct Database(pub sqlx::postgres::PgPool);

impl TypeMapKey for Database {
    type Value = Database;
}
