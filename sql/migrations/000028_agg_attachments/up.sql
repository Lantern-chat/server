CREATE OR REPLACE VIEW lantern.agg_attachments(
    msg_id,
    meta,
    preview
) AS

SELECT
    message_id as msg_id,
    jsonb_agg(json_build_object(
        'id', files.id,
        'size', files.size,
        'flags', files.flags,
        'name', files.name,
        'mime', files.mime
    )) AS meta,
    array_agg(files.preview) AS preview
FROM
    lantern.attachments INNER JOIN lantern.files ON files.id = attachments.file_id
GROUP BY
    msg_id
;