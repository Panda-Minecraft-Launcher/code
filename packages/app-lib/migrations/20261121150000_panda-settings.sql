ALTER TABLE settings ADD COLUMN sidebar_on_right INTEGER NOT NULL DEFAULT TRUE CHECK (sidebar_on_right IN (0, 1));
UPDATE settings SET compact_instance_cards = TRUE, show_screenshots_tab_in_instances = TRUE, refocus_on_game_close = TRUE, show_jump_in = FALSE;
