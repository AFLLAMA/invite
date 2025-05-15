// @generated automatically by Diesel CLI.

diesel::table! {
    presents (id) {
        id -> Int4,
        user_id -> Nullable<Int4>,
        name -> Text,
        link -> Text,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        name -> Text,
    }
}

diesel::joinable!(presents -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    presents,
    users,
);
