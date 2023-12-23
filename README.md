# Cloud Scraper

---
Get your cloud data on your terms.

## What is Cloud Scraper?

Cloud Scraper is an open source tool that allows you to download your data from cloud services on a regular basis to limit exposure to failure or data loss.
It's a command line tool that's easy to use, and can be used on any platform.

## What is the current status of Cloud Scraper?

Ideation/solution design - there is no working implementation at the time of writing.

Figure 1 shows the flow of data in the proposed solution. It uses Google as an example, but the same flow applies to any cloud service.

![Cloud Scraper context diagram](./diagrams/Context-Cloud_Scraper_Context.svg "Cloud Scraper")

*Figure 1 - Cloud Scraper context*

### Extensibility

The plan is to introduce pluggable modules for each cloud service, and a pluggable module for each data store. This will allow the user to choose which cloud service they want to use, and which data store they want to use.
