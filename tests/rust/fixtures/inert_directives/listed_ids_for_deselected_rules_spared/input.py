retries = [3, 4,]  # prose: skip[strip-trailing-commas]

connect(retries)  # prose: skip[align-equals, strip-trailing-commas]

TIMEOUT = 30  # prose: ignore[reassigned-constants]
TIMEOUT = 60

connect(TIMEOUT)  # prose: skip[align-equals]
