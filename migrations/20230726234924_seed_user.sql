-- Add migration script here
INSERT INTO users (user_id, username, password_hash)
VALUES (
    '8fbe7473-79b8-4641-a847-a35025337975',
    'admin',
    '$argon2id$v=19$m=15000,t=2,p=1$ncqQpyvJFworeoowkmGDfw$DnJTotAn4BapoeGNIwHZryZnQu1LNe6guj2NgRopXWU'
);
