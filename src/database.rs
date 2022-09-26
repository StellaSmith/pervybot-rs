use serenity::prelude::*;

pub struct Database(pub sqlx::any::AnyPool);

impl TypeMapKey for Database {
    type Value = Database;
}
