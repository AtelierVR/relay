namespace Relay.Instances;

public class UserModerated(uint id, string address) {
	public readonly uint            UserId  = id;
	public readonly string          Address = address;
	private         bool            _whitelisted;
	private         bool            _moderator;
	private         bool            _blacklisted;
	private         string?         _reason;
	private         DateTimeOffset? _expiration;

	public void SetBlacklist(bool active, string? reason = null, DateTimeOffset? expiresAt = null) {
		_blacklisted = true;
		_reason      = reason;
		_expiration  = expiresAt;
	}


	public bool IsBlacklisted(out string? reason, out DateTimeOffset? expiresAt) {
		if (_blacklisted) {
			if (_expiration.HasValue && _expiration.Value <= DateTimeOffset.UtcNow) {
				_blacklisted = false;
				_reason      = null;
				_expiration  = null;
				reason       = null;
				expiresAt    = null;
				return false;
			}

			reason    = _reason;
			expiresAt = _expiration;
			return true;
		}

		reason    = null;
		expiresAt = null;
		return false;
	}

	public void SetWhitelist(bool active)
		=> _whitelisted = active;

	public void SetModerator(bool active)
		=> _moderator = active;

	public bool IsWhitelisted()
		=> _whitelisted;

	public bool IsModerator()
		=> _moderator;
}