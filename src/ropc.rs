pub struct ROPC {
    client_id: ClientId,
    client_secret: ClientSecret,
    token_url: TokenUrl,
    token: CacheFile,
}

impl<T: Authorizer + Sync + Send> Authentication for ROPC {
    fn authorize(&self, builder: RequestBuilder) -> Result<RequestBuilder> {
        let token = self.token.fetch(|| self.ropc_flow())?;
        Ok(builder.bearer_auth(token))
    }
}
