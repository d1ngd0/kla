# AWS Sigv4 Signature

In order to use AWS Sigv4 with your requests you need to appropriately configure your machine. whether you have multiple AWS keys or not kla has your back.

KLA assumes you have set up your [Configuration and Credentials](https://docs.aws.amazon.com/cli/v1/userguide/cli-configure-files.html) files, but if not here is what you need.

In your `~/.aws/credentials` file you will want to add any keys you currently have

```
[default]
aws_access_key_id=ASIAIOSFODNN7EXAMPLE
aws_secret_access_key=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
aws_session_token = IQoJb3JpZ2luX2IQoJb3JpZ2luX2IQoJb3JpZ2luX2IQoJb3JpZ2luX2IQoJb3JpZVERYLONGSTRINGEXAMPLE

[user1]
aws_access_key_id=ASIAI44QH8DHBEXAMPLE
aws_secret_access_key=je7MtGbClwBF/2Zp9Utk/h3yCo8nvbEXAMPLEKEY
aws_session_token = fcZib3JpZ2luX2IQoJb3JpZ2luX2IQoJb3JpZ2luX2IQoJb3JpZ2luX2IQoJb3JpZVERYLONGSTRINGEXAMPLE
```

Within your `~/.aws/config` file you will want the appropriate settings for region.

```
[default]
region=us-west-2
output=json

[profile user1]
region=us-east-1
output=text
```

And that's it! if you run `kla --sigv4 /` the request will be signed using the `default` profile. If you want a different profile specify the `--sigv4-aws-profile` flag.

## Environments

These settings can be pre-configured for an environment as well!

```toml
[[environment]]
  name = "ceph"
  url = "http://cephrgw.example.com/admin/"

  # Specifies the --sigv4 flag should be enabled signing the http request with
  # amazons sigv4 https://docs.aws.amazon.com/IAM/latest/UserGuide/reference_sigv.html
  # sigv4_aws_profile specifies the profile to use and sigv4_aws_service defines the
  # service to use.
  # These settings assume you have set up your AWS credentials correctly. see
  # https://github.com/d1ngd0/kla/blob/main/docs/050_aws_settings.md
  sigv4 = true
  sigv4_aws_profile = "user1"
```

Any requests for this environment will have these settings enabled by default

```
kla --env ceph /info
```
