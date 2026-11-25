DELETE FROM install_job_requests
WHERE json_extract(request, '$.type') = 'install_optifine';

DELETE FROM install_jobs
WHERE kind = 'install_optifine'
	OR json_extract(state, '$.request.type') = 'install_optifine'
	OR EXISTS (
		SELECT 1
		FROM json_each(json_extract(state, '$.events')) AS event
		WHERE json_extract(event.value, '$.type') = 'job_queued'
			AND json_extract(event.value, '$.kind') = 'install_optifine'
	);

DELETE FROM instances
WHERE path LIKE '@optifine/%'
	OR id IN (
		SELECT instance_id
		FROM instance_content_sets
		WHERE loader = 'optifine'
	);

DELETE FROM instance_content_sets WHERE loader = 'optifine';

DELETE FROM synced_game_option_values
WHERE option_id LIKE 'optifine:%'
	OR raw_key LIKE 'optifine:%';

DELETE FROM synced_game_option_preferences WHERE option_id LIKE 'optifine:%';

DELETE FROM game_option_locale_origins
WHERE scope = 'optifine'
	OR scope LIKE 'optifine:%'
	OR option_id LIKE 'optifine:%';

DELETE FROM cache
WHERE id LIKE 'optifine:%'
	OR alias LIKE 'optifine:%';
