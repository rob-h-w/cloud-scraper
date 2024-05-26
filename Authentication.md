# Authentication

Google & similar online services are not set up to support easy authentication of headless
user-proxy software.

---

## Use Headless Browser?

Assuming the executing machine is not accessible by an adversary, storing user credentials, and
using a browser to login should be reasonable.

This would require a dependency on something like `chromium-chromedriver`.
