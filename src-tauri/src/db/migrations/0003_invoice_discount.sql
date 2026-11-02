-- A discount applied to the whole invoice.
--
-- `discount_kind` is "amount" (a fixed sum of money) or "percent" (a
-- percentage of the subtotal). `discount_value` holds what the user typed:
-- minor units for "amount", hundredths of a percent for "percent".
-- `discount_minor` is the discount the domain layer computed, so a stored
-- invoice always prints the numbers it was issued with.
--
-- The invoice total stays in `total_minor`; with a discount it holds the
-- amount actually due, so the history list shows what the customer pays.
ALTER TABLE invoices ADD COLUMN discount_kind TEXT NOT NULL DEFAULT 'amount';
ALTER TABLE invoices ADD COLUMN discount_value INTEGER NOT NULL DEFAULT 0;
ALTER TABLE invoices ADD COLUMN discount_minor INTEGER NOT NULL DEFAULT 0;
