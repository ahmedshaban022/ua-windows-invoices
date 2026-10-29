-- Custom invoice terms (بنود) chosen by the user for each invoice.
--
-- They are stored on the invoice itself, the same way the notes are: the
-- selection belongs to the document and must not change when the list of
-- terms in the settings changes later.
ALTER TABLE invoices ADD COLUMN terms_json TEXT NOT NULL DEFAULT '[]';
