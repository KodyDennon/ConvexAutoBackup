# Hosted Convex backup usage economics

Prices below are USD. Observed rates are dated. Scenario export sizes (200 MB, 2 GB, 20 GB) are planning assumptions from the research brief, not measured Convex deployments. No compression ratio is applied: the scenario figure is treated as the billed object size after the export ZIP is age-encrypted. Age overhead is assumed negligible (header bytes, well under 1 percent) and is not a compression assumption.

Operator cost here is object-storage cost of archives the operator stores. Convex database I/O and file egress are billed on the customer deployment that is exported. They are not operator COGS unless the operator chooses to absorb them. Compute to run the job and decrypt a restore test was not priced (see gaps).

## Current object-storage prices, including R2 egress

### Takeaway

As of the Cloudflare R2 pricing page last updated 1 October 2026, R2 Standard is $0.015 per GB-month, Class A is $4.50 per million requests, Class B is $0.36 per million, and egress to the internet is free when data leaves R2 directly. Infrequent Access is cheaper at rest ($0.01 per GB-month) but charges $0.01 per GB to read and bills a 30-day minimum. AWS S3 Standard in US East (N. Virginia), from the AWS price list published 28 September 2026, is $0.023 per GB-month for the first 50 TB. Writes are $0.005 per 1,000 requests and reads are $0.0004 per 1,000. Internet egress is not free: after a 100 GB per month account-wide allowance, US East (N. Virginia) charges $0.09 per GB for the next 10 TB (AWS Data Transfer price list published 16 September 2026). Backblaze B2 pay-as-you-go is $6.95 per TB per 30 days with free standard API calls and free egress up to 3× average stored bytes.

### Cited Findings

- R2 pricing page last updated 1 October 2026. Standard storage $0.015 per GB-month. Infrequent Access storage $0.01 per GB-month. Class A $4.50 per million (Standard) and $9.00 per million (Infrequent Access). Class B $0.36 per million (Standard) and $0.90 per million (Infrequent Access). Infrequent Access data retrieval $0.01 per GB. Egress to the internet listed as free for both classes, with footnote 1. — [Cloudflare R2 pricing](https://developers.cloudflare.com/r2/pricing/)
- Footnote 1: egress directly from R2, including the Workers API, the S3 API, and `r2.dev` domains, does not incur data-transfer charges. If other metered services are connected to the bucket, those services may charge. — [R2 pricing source](https://github.com/cloudflare/cloudflare-docs/blob/production/src/content/docs/r2/pricing.mdx)
- The same page states there are no egress-bandwidth charges for any storage class. The body text and the footnote disagree in scope: the body says egress is free; the footnote limits that to direct R2 egress and allows charges from other Cloudflare products attached to the bucket. — [Cloudflare R2 pricing](https://developers.cloudflare.com/r2/pricing/)
- R2 free tier, Standard only: 10 GB-month, 1 million Class A, 10 million Class B, per month. Infrequent Access is excluded. Egress remains the free footnote. — [Cloudflare R2 pricing](https://developers.cloudflare.com/r2/pricing/)
- R2 rounds usage up to the next billing unit. The documented examples: 1,000,001 operations billed as 2 million; 1.1 GB-month billed as 2 GB-month; 1.1 GB of Infrequent Access retrieval billed as 2 GB. — [Cloudflare R2 pricing](https://developers.cloudflare.com/r2/pricing/)
- R2 storage is the average of each day's peak over a 30-day bill, not the average byte-hour. A same-day extra copy raises that day's peak even if it is deleted later that day. — [Cloudflare R2 pricing](https://developers.cloudflare.com/r2/pricing/)
- R2 Class A includes `PutObject`, `CreateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload`, `ListObjects`, and `ListBuckets`, among others. Class B includes `GetObject` and `HeadObject`. `DeleteObject`, `DeleteBucket`, and `AbortMultipartUpload` are free. — [Cloudflare R2 pricing](https://developers.cloudflare.com/r2/pricing/)
- Infrequent Access minimum storage duration is 30 days. Objects deleted, moved, or replaced sooner are still billed for 30 days. Standard has no minimum duration. Retrieval fees apply whenever Infrequent Access data is read or copied. — [Cloudflare R2 pricing](https://developers.cloudflare.com/r2/pricing/)
- AWS S3 offer for us-east-1, price-list publication date 2026-09-28T23:04:16Z, version 20260928230416. S3 Standard `TimedStorage-ByteHrs`, General Purpose, US East (N. Virginia): $0.023 per GB for the first 50 TB per month (0–51,200 GB), $0.022 per GB for the next 450 TB (51,200–512,000 GB), $0.021 per GB above 500 TB. — [AWS S3 price list index](https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/AmazonS3/current/region_index.json) (region file `.../AmazonS3/20260928230416/us-east-1/index.json`)
- Same S3 price list: `Requests-Tier1` $0.005 per 1,000 PUT, COPY, POST, or LIST requests. `Requests-Tier2` $0.004 per 10,000 GET and all other requests, which is $0.0004 per 1,000 GET. Location US East (N. Virginia). — [AWS S3 price list](https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/AmazonS3/current/us-east-1/index.json)
- Same S3 price list: `DataTransfer-In-Bytes` $0 per GB ("S3-DT-AWS Inbound"). `DataTransfer-Out-Bytes` $0 per GB ("S3-DT-AWS Outbound"). `USE1-CloudFront-Out-Bytes` $0 per GB. These $0 lines are transfers to AWS or CloudFront, not internet egress. — [AWS S3 price list](https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/AmazonS3/current/us-east-1/index.json)
- AWS S3 pricing page prose (fetched 8 October 2026; the numeric HTML tables are client-rendered and were not in the fetched text): data transferred out to the internet for the first 100 GB per month is free, aggregated across AWS services and regions except China and GovCloud. Also free: data in from the internet, data between S3 buckets in the same region, data from S3 to any AWS service in the same region, and data out to CloudFront. — [Amazon S3 pricing](https://aws.amazon.com/s3/pricing/)
- AWS Data Transfer offer, us-east-1, publication date 2026-09-16T13:22:08Z, version 20260916132208. `DataTransfer-Out-Bytes` from US East (N. Virginia) to External: $0.090 per GB for the first 10 TB per month beyond the global free tier (0–10,240 GB), $0.085 per GB for the next 40 TB, $0.070 per GB for the next 100 TB, $0.050 per GB above 150 TB. — [AWS Data Transfer price list](https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/AWSDataTransfer/current/region_index.json)
- S3 pricing page DELETE and CANCEL requests are free. The page also describes separate retrieval fees for Standard-IA, One Zone-IA, and Glacier classes. Those classes are not used in the model below. — [Amazon S3 pricing](https://aws.amazon.com/s3/pricing/)
- The public S3 pricing page's Transfer Acceleration table lists $0.04 per GB for accelerated transfer via US, Europe, and Japan edge locations. That is Transfer Acceleration, not standard internet egress. — [Amazon S3 pricing](https://aws.amazon.com/s3/pricing/)
- Backblaze B2 pay-as-you-go is listed at $6.95 per TB per month, billed by byte-hour over the month at $6.95 per TB per 30 days. First 10 GB storage is free. Free egress up to 3× average monthly storage; above that, $0.01 per GB except unlimited free egress through listed CDN and compute partners. Standard API calls are free. No minimum file size or storage duration on pay-as-you-go. — [Backblaze B2 pricing](https://www.backblaze.com/cloud-storage/pricing)
- A 17 March 2026 Backblaze announcement, as mirrored the same day, said that effective 1 May 2026 pay-as-you-go storage would move from $6 per TB to $6.95 per TB and standard API calls would become free. The 3× egress allowance was unchanged. The mirror attributes the post to Backblaze. The original post URL was not fetched separately. — [Noise mirror, 17 March 2026](https://noise.getoto.net/2026/03/17/backblaze-pricing-and-product-updates/)
- A secondary write-up dated as of August 2026 repeats $6.95 per TB per 30 days, first 10 GB free, free uploads, free downloads up to 3× storage, then $0.01 per GB, and free Class A/B/C API calls. — [ZeroBuffer B2 pricing guide](https://www.zerobuffer.io/blogs/backblaze-b2-pricing)

### Inferences

- R2's zero-egress claim holds for the backup product if the worker uploads and downloads through the S3 API or the Workers API and does not put a metered product (a paid Workers route, a cache, or another Cloudflare service that bills bandwidth) in front of the bytes. A monthly restore download of the archive is $0 of R2 egress. Infrequent Access retrieval is a separate read fee and is not covered by the egress claim.
- R2 Infrequent Access is a poor fit for 14-day retention. Each object would still be billed for 30 days. It can fit a 30-day retention tier, with a $0.01 per GB charge on each restore read. The model uses Standard because the brief's short tiers (14 and 30 days) and restore tests make Standard the simpler default. At these request volumes the $0.005 per GB storage gap dominates the higher Infrequent Access request price.
- S3 internet egress is the main way S3 differs from R2 for this workload. Storage is only about 1.5× R2 ($0.023 / $0.015). A restore pulled to a non-AWS worker is $0.09 per GB after the account's 100 GB global allowance. A restore pulled by compute in the same region as the bucket is $0 egress under the S3 page exceptions. The model shows both.
- B2 storage is about $0.00695 per GB-month if $6.95 per TB is divided by 1,000 (decimal GB). That conversion is labeled. One restore per month is far below the 3× egress allowance, so restore egress is $0 and API calls are $0. B2 is cheaper storage than R2 for every scenario below. It is included because the brief asked whether it is relevant. It is.
- Account free tiers should not be built into a per-customer price. R2's 10 GB-month and S3's 100 GB egress allowance are once per operator account. Marginal cost after those allowances is the right COGS for a second customer. A brand-new account hosting only scenario 1 (2.8 GB-month, a 0.2 GB restore) pays about $0 on R2 and about $0 on S3 until the allowances are used.
- R2's round-up-to-the-next-GB rule is an account meter, not a per-customer meter. It adds at most one GB-month ($0.015) of storage error on the account total, plus a harsh operations cliff: 1,000,001 Class A requests are billed as 2 million. The scenarios below are thousands of writes, so they sit inside the first million and inside the Class A free tier. List-price operation costs are shown anyway so margin does not depend on the free tier.

### Gaps

- Inter-region AWS transfer rates were not extracted. The S3 offer contains many `S3RTC` and cross-region out SKUs. The model uses same-region $0 or internet $0.09 only. A worker and a bucket in different regions would add a third egress case.
- Cloudflare's published rounding examples do not say whether the 10 GB free tier is applied before or after the round-up. For a multi-tenant account the difference is at most one GB-month.
- No primary page was fetched that restates B2's $6.95 rate with a "last updated" stamp in the page body. The figure is from Backblaze's pricing page as retrieved via search on 8 October 2026, consistent with the 17 March 2026 announcement of the 1 May 2026 change.

## Realistic Convex deployment sizes

### Takeaway

Convex does not publish a distribution of production deployment sizes. Documented caps put a solo app in the hundreds of megabytes, a typical paid app inside Professional's included 50 GB of database and 100 GB of files, and a file-heavy app at tens of gigabytes still under the 100 GB included file allowance and far under the 1 TB serverless dataset cap. The brief's 200 MB, 2 GB, and 20 GB exports sit inside those documented bounds. They are assumptions, not measurements. Public evidence of an actual export is a single image over 500 MB and a table of 10 million documents, with no byte total and no duration.

### Cited Findings

- `npx convex export` writes a ZIP of table documents at `<table_name>/documents.jsonl`. `--include-file-storage` adds a `_storage` folder. Default target is the dev deployment; `--prod` selects production. — [Convex export CLI](https://docs.convex.dev/cli/reference/export)
- A backup is a consistent snapshot and can include file storage. Dashboard manual backups are kept 7 days. Scheduled daily backups are kept 7 days. Weekly backups are kept 14 days. Free and Starter deployments can store at most two backups at a time. Professional can store many, on usage pricing. — [Convex backup and restore](https://docs.convex.dev/database/backup-restore)
- Backup pricing, from that page: backups use database bandwidth to read all documents and file bandwidth to include user files. Generation and storage of the backup are billed at the same bandwidth and storage prices as user file storage, visible on the usage dashboard. — [Convex backup and restore](https://docs.convex.dev/database/backup-restore)
- The ZIP export guide says you can export by taking a backup and downloading it, or export the same data with `npx convex export`. It also mentions a separate paginated streaming export via a Data Sync API. — [Convex data export](https://docs.convex.dev/database/import-export/export)
- Limits page, fetched 8 October 2026. The page says quoted prices are for US regions and other regions are 1.3×. It does not show its own "last updated" date. — [Convex limits](https://docs.convex.dev/production/state/limits)
- Documents are limited to 1 MiB, including system fields. — [Convex limits](https://docs.convex.dev/production/state/limits)
- Database storage includes rows and indexes, not files or backups. Each index is priced as another copy of the table. Free: 0.5 GB total. Starter: 0.5 GB included, then $0.22 per GB-month. Professional: 50 GB included, then $0.20 per GB-month. Business and Enterprise serverless: $0.20 per GB-month. Dedicated: $0.45 per GB-month. — [Convex limits](https://docs.convex.dev/production/state/limits)
- Database I/O is document and index data moved between Convex functions and the database. Free: 1 GB per month total. Starter: 1 GB included, then $0.22 per GB. Professional: 50 GB included, then $0.20 per GB. Business and Enterprise serverless: $0.20 per GB. Dedicated: $0.15 per GB. — [Convex limits](https://docs.convex.dev/production/state/limits)
- File storage includes user files and backups. Free: 1 GB total. Starter: 1 GB included, then $0.033 per GB-month. Professional: 100 GB included, then $0.03 per GB-month. Business and Enterprise: $0.03 per GB-month. — [Convex limits](https://docs.convex.dev/production/state/limits)
- File data egress includes serving user files, reading user files inside functions, and generating and restoring backups. Free: 1 GB per month total. Starter: 1 GB included, then $0.132 per GB. Professional: 50 GB included, then $0.12 per GB. Business and Enterprise: $0.12 per GB. — [Convex limits](https://docs.convex.dev/production/state/limits)
- There is no per-file size cap on storage. A direct upload POST times out at 2 minutes. HTTP actions are limited to a 20 MB request body. Upload URLs are the path for larger files. — [Convex file storage](https://docs.convex.dev/file-storage/upload-files)
- Serverless deployment classes S16 (Free, Starter, and an option on Business) and S256 (Professional) have a max dataset of 1 TB. Dedicated D1024 is 4 TB and D2048 is 8 TB. S16 and S256 backups are logical. Dedicated backups are high-speed physical, not downloadable, and included in the hardware price. Physical restores go through Convex support. — [Convex limits](https://docs.convex.dev/production/state/limits)
- Convex 1.9 added file storage to snapshot export and import, and changed import so it is no longer one HTTP request that timed out above 10 MB. The post says gigabytes of documents and files can be imported with progress messages, that one image over 500 MB works with snapshot export and import, and that a table named "acquaintances" has 10 million documents. No byte total and no elapsed time are given. — [Announcing Convex 1.9](https://news.convex.dev/announcing-convex-1-9/)
- An Octopus Deploy step template defaults `npx convex export` to a 600 second command timeout and says large datasets may need a higher value. That timeout is the integrator's, not a Convex limit. — [Octopus Convex export](https://octopus.com/integrations/convex/convex-export-data)
- A self-hosted bug report (April 2026) says `npx convex export --include-file-storage` grows RAM on self-hosted backends. The write-up mentions 128 concurrent file fetches and a 32 MB inflight prefetch cap in the export path. It is a self-hosted memory leak, not a cloud duration benchmark. — [get-convex/convex-backend issue 435](https://github.com/get-convex/convex-backend/issues/435)

### Inferences

- A 200 MB export is a small app. It fits in Starter's included 1 GB of files if most of the ZIP is files, or in a fraction of Professional's 50 GB database if most of it is documents. Free's hard 1 GB file cap and 0.5 GB database cap mean a 200 MB production export is plausible on Free only when documents plus files stay under those caps. One full copy is not the same as Convex's stored size: indexes are billed as extra copies of a table, and the export JSONL is one logical copy of documents.
- A 2 GB export is a small production database or a light file store. It is 4 percent of Professional's included 50 GB of database, or 2 percent of the included 100 GB of files. It does not require dedicated hardware. It is above Free's hard caps if it is a single deployment's live data.
- A 20 GB export is file-heavy but still inside Professional's included 100 GB of file storage, and far inside the 1 TB serverless dataset cap. It is a normal Convex app size. It is a painful backup size only because every run re-reads all of it (next sections).
- Ten million documents do not imply a multi-gigabyte database. At the 1 MiB document cap they would be 10 TB, which exceeds the 1 TB serverless dataset cap, so that demo table's documents were much smaller. The post does not say how small. No average document size is inferred here.
- Convex's own backup product keeps dailies for 7 days and weeklies for 14 days, and stores that copy as file storage. A hosted product that keeps 14 to 30 copies off Convex is selling longer retention and an off-platform copy, not a cheaper way to read the data. The read is billed either way.

### Gaps

- No public histogram, survey, or status-page sample of Convex deployment database or file-storage bytes was found. The 200 MB / 2 GB / 20 GB figures must stay labeled as scenarios.
- No primary source states how many seconds a cloud export of a given size takes, or the maximum ZIP Convex will produce.
- It is not explicit whether `npx convex export` leaves a second copy in Convex file storage the way a dashboard backup does, or only charges the read. The backup page bills "generation and storage" for backups. The export guide treats the CLI as another way to produce the same ZIP. Storage of a Convex-side copy is not added to operator COGS below. Customer read bandwidth is estimated as one full pass over the exported bytes.

## Cost model for four usage scenarios

### Takeaway

On R2 Standard, ignoring account free tiers, steady-state storage plus a 5 percent full-upload retry and one monthly restore is about $0.04 per month for a 200 MB daily backup kept 14 copies, about $1.15 for a 2 GB deployment with 30 daily copies plus 8 weekly copies, about $4.26 for a 20 GB daily backup kept 14 copies, and about $12.12 for an assumed agency of 15 small deployments and 10 typical ones ($0.48 average per deployment). Request charges are cents. S3 storage is about 1.5× those figures. S3 internet egress on the single monthly restore adds $0.02, $0.18, and $1.80 for the three sizes, and $0 if the restore runs in the same AWS region. The customer's Convex read, which the operator does not pay, is larger than operator storage once a multi-gigabyte export runs every day.

### Cited Findings

- Storage and request rates used below are the R2, S3, AWS Data Transfer, and B2 rates cited in the first section. Convex overage rates are the US Professional and Starter rates cited in the second section.
- R2 Class A explicitly includes multipart create, upload-part, and complete. S3 Tier1 is "PUT, COPY, POST, or LIST." The model counts each of those multipart calls as one write. That S3 mapping is a modeling assumption aligned with the request categories, not a separate SKU quote for `UploadPart`. — [Cloudflare R2 pricing](https://developers.cloudflare.com/r2/pricing/); [AWS S3 price list](https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/AmazonS3/current/us-east-1/index.json)

### Inferences

Labeled modeling choices, applied to every row:

- Month length: 30 days. Daily schedules run 30 times. Weekly schedules run 4 times. Retention is a copy count, so using 4.33 weekly runs (52/12) would not change GB-month. It would add about 8 percent more weekly PUTs, which is still under $0.01.
- Steady state: export size is constant. GB-month equals copies retained times export GB. Daily and weekly are separate schedules, so scenario 2 stores 30 + 8 = 38 copies, not 30. If the weekly tier only keeps 8 of the daily objects, storage falls to 30 copies (60 GB-month, R2 $0.90 before retries). The higher figure is the one used.
- One destination. A second destination stores another full set of copies. Convex is read once if the worker fans out after a single export. Operator storage and PUTs scale with destinations. The restore test is one download, not one per destination.
- Multipart: objects at or under 100 MiB would be one `PutObject`. All three scenario sizes are above that, so parts are 64 MiB. Write ops per object = ceil(decimal_bytes / 64 MiB) + 2 (create and complete). 0.2 GB → 3 parts + 2 = 5 writes. 2 GB → 30 parts + 2 = 32 writes. 20 GB → 299 parts + 2 = 301 writes. One `ListObjects` per job is an extra Class A / Tier1 call. Deletes are free and are not counted.
- If the implementation instead used a single PUT per archive, write ops would fall to one per job plus the list. The R2 Class A bill for scenario 3 would drop by about $0.04. Operation cost is not sensitive to part size at these archive sizes.
- Retries: 5 percent of jobs fail after the object is fully written, then succeed on one retry. That is an assumption, not a measured failure rate. Extra writes = 0.05 × jobs × writes per object. Because R2 bills the day's peak, each retry also adds export_GB / 30 GB-month. A failure before any PUT, followed by `AbortMultipartUpload`, has about $0 operator storage cost (abort is free) but still costs the customer another Convex read if the export was regenerated.
- Restore test: once per deployment per month, download the newest archive (one GET) and decrypt it. Decrypt CPU is not given a dollar figure.
- Dollars use list rates with no free tier, so they stay valid after the first customers fill the 10 GB R2 allowance and the 100 GB S3 egress allowance. Operation totals are shown at list price even though they are inside R2's 1 million Class A free tier.
- B2 uses $6.95 / 1,000 = $0.00695 per GB-month. API calls $0. Restore egress $0 because one download is under 3× stored bytes.
- Customer Convex dollars assume the exported bytes are entirely files (file egress) or entirely documents (database I/O). The real ZIP is a mix. Both ends are shown so the mix can be scaled. Professional included 50 GB is subtracted in the "allowance still unused" column and ignored in the "allowance already used by the app" column. Starter's 1 GB egress allowance is the comparable pair for small plans. Other regions would be 1.3× these Convex rates.

Operator cost, one destination, list price, 5 percent retry included:

| Scenario | Copies | Storage GB-month | Jobs / month | Write ops / month, including lists and retries | R2 total | S3 storage + requests | S3 plus internet egress on the restore | B2 storage |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1. Solo, 0.2 GB, daily, keep 14 | 14 | 2.80 | 30 | 189 | $0.043 | $0.066 | $0.084 | $0.020 |
| 2. Typical, 2 GB, daily keep 30 plus weekly keep 8 | 38 | 76.00 | 34 | 1,178 | $1.15 | $1.76 | $1.94 | $0.53 |
| 3. File-heavy, 20 GB, daily, keep 14 | 14 | 280.00 | 30 | 9,513 | $4.26 | $6.51 | $8.31 | $1.95 |
| 4. Agency, 15 × scenario 1 and 10 × scenario 2 | mixed | 802.00 | 790 | 14,615 | $12.12 | $18.55 | $20.62 | $5.58 |

Agency average per deployment at that mix: R2 $0.48, S3 $0.74, S3 with restore egress $0.82, B2 $0.22. Bounds if all 25 deployments are the same shape: 25 × scenario 1 is R2 $1.08; 25 × scenario 2 is R2 $28.68. The 15/10 split is an assumption.

Arithmetic for the R2 storage line, before the small retry peak:

- Scenario 1: 14 × 0.2 GB = 2.8 GB-month × $0.015 = $0.042. Retry peak 1.5 × 0.2 / 30 = 0.01 GB-month × $0.015 = $0.00015. Class A about 189 / 1,000,000 × $4.50 = $0.00085. One Class B GET is $0.00000036. Total $0.043.
- Scenario 2: (30 + 8) × 2 GB = 76 GB-month × $0.015 = $1.14. Retry peak 1.7 × 2 / 30 = 0.113 GB-month × $0.015 = $0.0017. Class A about 1,178 / 1,000,000 × $4.50 = $0.0053. Total $1.15.
- Scenario 3: 14 × 20 GB = 280 GB-month × $0.015 = $4.20. Retry peak 1.5 × 20 / 30 = 1.0 GB-month × $0.015 = $0.015. Class A about 9,513 / 1,000,000 × $4.50 = $0.043. Total $4.26.
- Scenario 4: 15 × 2.8 + 10 × 76 = 802 GB-month. 15 × $0.043 + 10 × $1.15 = $12.12.

S3 storage check: multiply the R2 storage GB-month (including retry peak) by $0.023. Scenario 1: 2.81 × $0.023 = $0.065, plus about $0.001 of requests, plus 0.2 × $0.09 = $0.018 egress. Scenario 2: 76.11 × $0.023 = $1.75, plus 2 × $0.09 = $0.18 egress. Scenario 3: 281 × $0.023 = $6.46, plus 20 × $0.09 = $1.80 egress.

Same-region S3 restore drops the egress column to the "storage + requests" column. The first 100 GB per month of internet egress is $0 for the whole AWS account. Scenario 4's restores download 15 × 0.2 + 10 × 2 = 23 GB, which fits in that allowance if the account has no other internet egress. The table's higher S3 column assumes the allowance is already gone.

Two destinations, storage-dominated, about 2× the R2 column: scenario 2 about $2.29, scenario 3 about $8.52. Convex read bandwidth does not double.

Customer Convex read, not in the operator column. Professional US. Jobs are 30, 34, and 30 as above. Bytes per month = export GB × jobs.

| Scenario | GB read per month | File egress if the 50 GB allowance is still free | File egress if the allowance is already used | Database I/O if the 50 GB allowance is still free | Database I/O if the allowance is already used |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 6 | $0 | 6 × $0.12 = $0.72 | $0 | 6 × $0.20 = $1.20 |
| 2 | 68 | (68 − 50) × $0.12 = $2.16 | 68 × $0.12 = $8.16 | (68 − 50) × $0.20 = $3.60 | 68 × $0.20 = $13.60 |
| 3 | 600 | (600 − 50) × $0.12 = $66.00 | 600 × $0.12 = $72.00 | (600 − 50) × $0.20 = $110.00 | 600 × $0.20 = $120.00 |

Starter file egress is $0.132 per GB after 1 GB. The same three read volumes cost about (6 − 1) × $0.132 = $0.66, (68 − 1) × $0.132 = $8.84, and (600 − 1) × $0.132 = $79. A 20 GB file deployment also exceeds Starter's 1 GB included file storage by 19 GB × $0.033 = $0.63 per month of live storage, separate from the backup reads. Free cannot hold a 20 GB file store (1 GB hard cap) or a 2 GB document store (0.5 GB hard cap).

Retries add about 5 percent to the customer read column if a failed job re-exports from Convex. That is $0.04 to $3.60 of extra Professional file egress on scenario 3, depending on whether the allowance is already used. It does not meaningfully change operator storage.

### Gaps

- No measured retry rate. Five percent is a planning assumption. Pre-upload failures cost the operator almost nothing and cost the customer another read.
- Worker, disk, and age-decrypt CPU are not priced. A shared worker is likely to be small next to scenario 2 and 3 storage. A dedicated virtual machine per solo deployment would cost more than the $0.04 of R2. No current VM price was pulled, so this stays qualitative.
- The document-versus-file split inside a real export is unknown, so customer Convex cost is a range, not a point.
- Dashboard backup storage (a Convex-side copy kept 7 or 14 days, billed as file storage) is not added. Using both Convex's scheduler and this product would add that storage on the customer bill.

## When full exports become a bad product

### Takeaway

Operator storage stays cheap well into tens of gigabytes. The product becomes a bad deal for the customer when a full read is repeated on a schedule, because Convex charges file egress at $0.12 per GB and database I/O at $0.20 per GB on Professional after 50 GB. A 20 GB daily export is about $66 to $72 per month of Convex file egress before the operator's $4 R2 bill. No public source gives an export timeout or a throughput, so a size where jobs start overlapping the next interval cannot be calculated from evidence. Weekly full copies, or a real incremental format, are what keep large file stores viable. Incremental backup would cut stored bytes and Convex reads by roughly the ratio of the change rate to the full size. A labeled 5 percent daily change illustration is about a 12× storage reduction versus 30 full copies. Convex has no incremental export today.

### Cited Findings

- There is no incremental or diff export in the product under study. The brief states that, and the export docs describe a full ZIP snapshot only, plus an optional paginated streaming export that is still a full read. — [Convex data export](https://docs.convex.dev/database/import-export/export); [Convex export CLI](https://docs.convex.dev/cli/reference/export)
- Convex's own dailies are retained 7 days and weeklies 14 days. Dedicated physical backups are not downloadable. — [Convex backup and restore](https://docs.convex.dev/database/backup-restore); [Convex limits](https://docs.convex.dev/production/state/limits)
- File egress and database I/O prices are the Professional US rates in the previous section. — [Convex limits](https://docs.convex.dev/production/state/limits)
- The only duration-related public number found is the Octopus integrator's default 600 second timeout, with a note that large datasets may need longer. Convex 1.9 shows that a multi-hundred-megabyte file and a 10 million document table can be exported, without a clock time. The import path used to fail above 10 MB as a single HTTP request; that limit was removed for import in 1.9. — [Octopus Convex export](https://octopus.com/integrations/convex/convex-export-data); [Announcing Convex 1.9](https://news.convex.dev/announcing-convex-1-9/)
- User-function limits (1 second queries, 10 minute Node actions, 30 minute Convex runtime actions) are documented. They govern user code, not the export pipeline. — [Convex limits](https://docs.convex.dev/production/state/limits)

### Inferences

- Cost, not a documented timeout, is the first hard stop. Compare scenario 3: operator R2 about $4.26 per month, customer Convex file egress about $66 to $72 per month. The customer pays an order of magnitude more to Convex to produce the backup than the operator pays to keep 14 copies. Raising the operator's price does not fix that.
- A weekly 20 GB export kept 8 copies would store 160 GB-month, R2 160 × $0.015 = $2.40, and read 4 × 20 = 80 GB from Convex. Professional file-egress overage if the 50 GB allowance is otherwise unused: (80 − 50) × $0.12 = $3.60, versus $66 for the daily schedule. Same archive size, much less pain. Frequency is the lever while exports are full copies.
- Scenario 2 at 2 GB daily plus weekly is still a reasonable customer bill: about $2 to $8 of Professional file egress, or $4 to $14 of database I/O, plus about $1.15 of operator R2. Scenario 1 is noise on both sides.
- A flat daily product sold to file-heavy apps without a size cap will attract the deployments where Convex egress already looks like a bug. Native Convex backups have the same read price and shorter retention, so the hosted product's extra value is the longer off-platform history, not a discount on the read.
- Job overlap is a real failure mode once export duration exceeds the interval (24 hours for daily, 7 days for weekly). No source supports a bytes-per-second figure, so no GB threshold is stated. The 10 minute Octopus default would be uncomfortable for multi-gigabyte file exports if cloud export were that slow, but that default is not evidence of Convex's speed.
- Multiple destinations multiply operator storage and the customer's pain only if each destination triggers its own export. One export fanned out to several buckets multiplies storage, not Convex reads.
- What incremental backup would change, illustrated with an assumed 5 percent of bytes changing per day. This change rate is not a Convex measurement. For 30 days of history, full copies store 30 × F. One base plus 29 diffs store F × (1 + 29 × 0.05) = 2.45 × F, about 12× less. At 2 GB, that is 4.9 GB-month instead of 60 GB-month of dailies, R2 about $0.07 instead of $0.90. At 20 GB and 30 copies, 49 GB-month instead of 600, R2 about $0.74 instead of $9.00. Convex reads would fall from a full copy every day to about the change rate, plus an occasional new base. A monthly restore would download the base plus the diffs needed to reconstruct one day, so restore egress stays on the order of one full copy, not 12× smaller. Incremental backup fixes the customer's daily Convex bill and the operator's retention bill. It does not remove the need to read a full archive on restore.

### Gaps

- No published export throughput, cloud timeout, or maximum archive size. The overlap threshold is unknown.
- No evidence for a typical daily change rate of Convex documents or files. The 5 percent figure is only an illustration.
- Streaming export via the Data Sync API might avoid a single ZIP, but no pricing or duration comparison versus `npx convex export` was found. It would still be a full read on current docs.

## Prices that leave 70 percent gross margin

### Takeaway

Gross margin here is (price − operator object-storage COGS) / price, using the R2 list-price totals above (storage, retries, restore GET, no egress). Seventy percent margin means price at least COGS / 0.30. That floor is about $0.15 per month for the solo deployment, $3.83 for the typical deployment, $14.20 for the 20 GB daily deployment, and $1.62 average for the assumed agency mix. A flat $9 per deployment per month clears 70 percent on scenarios 1 and 2 (margins about 99 percent and 87 percent) and fails scenario 3 (margin about 53 percent). A flat $15 clears scenario 3 on R2 (about 72 percent) and fails it if the customer has two destinations or the operator is on S3 with internet restore egress. The storage margin breaks when retained GB-month exceeds 0.30 × price / $0.015 on R2. It breaks earlier as a product, even with healthy operator margin, when the customer's Convex read bill dominates.

### Cited Findings

- COGS figures are the R2, S3, and B2 totals derived in the previous section from the cited list prices. No additional price-list fact is introduced here.

### Inferences

Minimum monthly price per deployment for 70 percent margin after storage COGS only:

| Scenario | R2 COGS | Minimum price on R2 | S3 COGS, same-region restore | Minimum price | S3 COGS, internet restore | Minimum price |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1. Solo | $0.043 | $0.15 | $0.066 | $0.22 | $0.084 | $0.28 |
| 2. Typical | $1.15 | $3.83 | $1.76 | $5.86 | $1.94 | $6.46 |
| 3. File-heavy | $4.26 | $14.20 | $6.51 | $21.70 | $8.31 | $27.70 |
| 4. Agency average (15 + 10 mix) | $0.48 | $1.62 | $0.74 | $2.47 | $0.82 | $2.75 |

The agency floor is an average. A flat price must clear scenario 2 ($3.83 on R2), not the $1.62 blend, or the larger deployments in the agency are under 70 percent while the small ones subsidize the average.

Flat R2 margin at a few round prices, one destination:

- $5: scenario 1 about 99 percent, scenario 2 about 77 percent, scenario 3 about 15 percent.
- $9: scenario 1 about 99.5 percent, scenario 2 about 87 percent, scenario 3 about 53 percent.
- $15: scenario 3 about 72 percent. This is the first round price in this set that clears 70 percent for a 20 GB daily, 14-copy deployment on R2.
- $19: scenario 3 about 78 percent on R2. On S3 with a $0.09 restore (COGS $8.31), margin is (19 − 8.31) / 19 = 56 percent, under 70 percent.
- $29: scenario 3 on S3 with internet restore, (29 − 8.31) / 29 = 71 percent.

Where a flat R2 price stops clearing 70 percent, ignoring the cent-level request and retry terms. Maximum export size ≈ (0.30 × price) / ($0.015 × copies).

- $9, keep 14: 12.9 GB. Keep 30: 6.0 GB. Keep 38 (30 daily + 8 weekly of the same size): 4.7 GB.
- $15, keep 14: 21.4 GB. Keep 30: 10.0 GB. Keep 38: 7.9 GB.
- $19, keep 14: 27.1 GB. Keep 30: 12.7 GB. Keep 38: 10.0 GB.
- $29, keep 14: 41.4 GB. Keep 30: 19.3 GB. Keep 38: 15.3 GB.

Two destinations roughly halve those size caps, because COGS doubles. Scenario 3 at two destinations is about $8.52 R2, which needs a price of $8.52 / 0.30 = $28.40 to hold 70 percent. A $15 price on that deployment is a storage margin of (15 − 8.52) / 15 = 43 percent.

B2 at $0.00695 per GB-month lowers every floor by about 0.015 / 0.00695 ≈ 2.2×. Scenario 3's B2 COGS of $1.95 needs only about $6.50 to clear 70 percent. Choosing R2 over B2 is a margin choice, not a technical requirement of the backup format, as long as restore volume stays inside B2's 3× egress allowance. One restore per month does.

These margins are not a full gross margin. They exclude payment fees, support, and the worker. They also exclude Convex bandwidth, which is the customer's invoice, not the operator's, when the export uses the customer's deploy key. Scenario 3 can show a 72 percent operator margin at $15 and still be a bad purchase, because the customer is paying Convex about $66 to $72 to generate what the operator stores for $4.26. The price that protects operator margin and the price the customer can justify are different constraints. The customer constraint binds first on large daily full exports.

### Gaps

- No competitor backup-SaaS price for Convex was found, so these floors are not positioned against a market rate.
- Support time, failed-job paging, and payment-processing fees are outside the storage COGS. A solo customer at $5 with $0.04 of R2 still fails 70 percent margin if support or a dedicated runner costs more than $1.50.
