# Authentication

Google & similar online services are not set up to support easy authentication of headless
user-proxy software.

The intended uses for Google APIs is to construct multi-tenant services for users. Single-tenant
proxies like Cloud Scraper are not the intended use-case.

---

## Use Headless Browser?

Assuming the executing machine is not accessible by an adversary, storing user credentials, and
using a browser to login should be reasonable.

This would require a dependency on something like `chromium-chromedriver`, and would be
susceptible to changes to the online service's site content. In the worst case, Cloud Scraper
might be identified as a bot and blocked.

## Use OAuth2?

OAuth2 is a standard for authentication, and is supported by Google APIs. It is designed for
multi-tenant systems, and so requires any user of Cloud Scraper to have a developer account with
the OAuth2 provider.

### Maybe Support Federation?

If Cloud Scraper were to support OAuth2, it could support federation. This would enable the user to
use fewer Oauth2 accounts to access multiple services.
