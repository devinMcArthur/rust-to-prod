-- Add migration script here
-- We are going to add to this table even before we know the status of a call,
-- therefore we are going to relax the necessity of adding response details
ALTER TABLE idempotency ALTER COLUMN response_status_code DROP NOT NULL;
ALTER TABLE idempotency ALTER COLUMN response_body DROP NOT NULL;
ALTER TABLE idempotency ALTER COLUMN response_headers DROP NOT NULL;
