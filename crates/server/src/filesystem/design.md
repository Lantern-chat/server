NOTES:
* Up to N filles should be uploaded or downloaded to cold-storage concurrently, keep a semaphore to limit that
* If a file is being downloaded from cold-storage, block multiple requests for that file until it's downloaded
    * Use some kind of `CHashMap<FileId, Mutex>` for that?
    * Make sure it doesn't attempt to download a file more than once