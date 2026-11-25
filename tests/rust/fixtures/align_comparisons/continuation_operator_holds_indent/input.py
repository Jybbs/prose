if (
    collation is None
    or "supports_collation"
    in model.required_features
):
    pass
