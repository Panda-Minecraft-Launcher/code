DELETE FROM install_jobs
WHERE state LIKE '%optifine%' COLLATE NOCASE
	OR id IN (
		SELECT job_id
		FROM install_job_requests
		WHERE request LIKE '%optifine%' COLLATE NOCASE
	);

DELETE FROM install_job_requests
WHERE request LIKE '%optifine%' COLLATE NOCASE;
