CREATE OR REPLACE VIEW lantern.agg_attachments(
    msg_id,
    meta,
    preview
) AS

SELECT
    message_id as msg_id,
    jsonb_agg(jsonb_build_object(
        'id', files.id,
        'size', files.size,
        'flags', files.flags,
        'name', files.name,
        'mime', files.mime,
        'width', files.width,
        'height', files.height
    )) AS meta,
    array_agg(files.preview) AS preview
FROM
    lantern.attachments INNER JOIN lantern.files ON files.id = attachments.file_id
WHERE
    attachments.flags & 1 = 0 -- where not orphaned
GROUP BY
    msg_id
;