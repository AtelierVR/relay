using System.Numerics;

namespace Relay.Utils {
	public class Buffer(byte[] data) {
		public const int DefaultLength = Constants.MaxPacketSize;

		private byte[] _data   = data;
		private int    _length = data.Length;
		private int    _offset = 0;

		public static Buffer New(int length = DefaultLength)
			=> new(new byte[length]) { _length = 0 };


		public int Length
			=> _length;

		public int Offset
			=> _offset;

		public bool Write(byte value) {
			if (_offset + 1 > _data.Length) return false;
			_data[_offset++] = value;
			if (_offset > _length) _length = _offset;
			return true;
		}

		public bool Write(short value) {
			if (_offset + 2 > _data.Length) return false;
			_data[_offset++] = (byte)(value >> 8);
			_data[_offset++] = (byte)value;
			if (_offset > _length) _length = _offset;
			return true;
		}

		public bool Write(ushort value)
			=> Write((short)value);

		public bool Write(string value) {
			if (value == null || _offset + value.Length + 2 > _data.Length) return false;
			if (!Write((ushort)value.Length)) return false;
			foreach (var c in value)
				_data[_offset++] = (byte)c;
			if (_offset > _length) _length = _offset;
			return true;
		}

		public bool Write(DateTimeOffset value)
			=> Write(value.ToUnixTimeMilliseconds());

		public bool Write(int value) {
			if (_offset + 4 > _data.Length) return false;
			_data[_offset++] = (byte)(value >> 24);
			_data[_offset++] = (byte)(value >> 16);
			_data[_offset++] = (byte)(value >> 8);
			_data[_offset++] = (byte)value;
			if (_offset > _length) _length = _offset;
			return true;
		}

		public bool Write(uint value)
			=> Write((int)value);

		public bool Write(long value) {
			if (_offset + 8 > _data.Length) return false;
			_data[_offset++] = (byte)(value >> 56);
			_data[_offset++] = (byte)(value >> 48);
			_data[_offset++] = (byte)(value >> 40);
			_data[_offset++] = (byte)(value >> 32);
			_data[_offset++] = (byte)(value >> 24);
			_data[_offset++] = (byte)(value >> 16);
			_data[_offset++] = (byte)(value >> 8);
			_data[_offset++] = (byte)value;
			if (_offset > _length) _length = _offset;
			return true;
		}

		public bool Write(ulong value)
			=> Write((long)value);

		public bool Write(float value) {
			if (_offset + 4 > _data.Length) return false;
			var bytes = BitConverter.GetBytes(value);
			if (BitConverter.IsLittleEndian) Array.Reverse(bytes);
			foreach (var b in bytes)
				_data[_offset++] = b;
			if (_offset > _length) _length = _offset;
			return true;
		}


		public bool Write(double value) {
			if (_offset + 8 > _data.Length) return false;
			var bytes = BitConverter.GetBytes(value);
			if (BitConverter.IsLittleEndian) Array.Reverse(bytes);
			foreach (var b in bytes)
				_data[_offset++] = b;
			if (_offset > _length) _length = _offset;
			return true;
		}

		public bool Write(Vector3 value) {
			if (_offset + 12 > _data.Length) return false;
			if (!Write(value.X)) return false;
			if (!Write(value.Y)) return false;
			if (!Write(value.Z)) return false;
			if (_offset > _length) _length = _offset;
			return true;
		}

		public bool Write(Quaternion value) {
			if (_offset + 16 > _data.Length) return false;
			if (!Write(value.X)) return false;
			if (!Write(value.Y)) return false;
			if (!Write(value.Z)) return false;
			if (!Write(value.W)) return false;
			if (_offset > _length) _length = _offset;
			return true;
		}

		public bool Write(byte[] value) {
			if (_offset + value.Length > _data.Length) return false;
			foreach (var b in value)
				_data[_offset++] = b;
			if (_offset > _length) _length = _offset;
			return true;
		}

		public byte[] ToBuffer() {
			var buffer = new byte[_length];
			System.Buffer.BlockCopy(_data, 0, buffer, 0, _length);
			return buffer;
		}

		public void Goto(ushort offset)
			=> _offset = offset;

		public void GotoEnd()
			=> _offset = _length;

		public void Skip(ushort offset)
			=> _offset += offset;

		public override string ToString() {
			var res = $"Buffer[(offset={_offset.ToString()}, length={_length.ToString()}) ";
			try {
				for (var i = 0; i < _length; i++) res += (i == 0 ? "" : " ") + _data[i].ToString("X2");
			} catch (Exception e) {
				Logger.Error(e.ToString());
			}

			return res + "]";
		}

		public byte ReadByte() {
			if (_offset + 1 > _length) return 0;
			return _data[_offset++];
		}

		public ushort ReadUShort() {
			if (_offset + 2 > _length) return 0;
			var value = (ushort)(_data[_offset++] << 8);
			value |= _data[_offset++];
			return value;
		}

		public string? ReadString() {
			var length = ReadUShort();
			if (_offset + length > this._length) return null;
			var value = System.Text.Encoding.UTF8.GetString(_data, _offset, length);
			_offset += length;
			return value;
		}

		public DateTime ReadDateTime() {
			var value = ReadLong();
			return DateTimeOffset.FromUnixTimeMilliseconds(value).DateTime;
		}

		public int ReadInt() {
			if (_offset + 4 > _length) return 0;
			var value = _data[_offset++] << 24;
			value |= _data[_offset++] << 16;
			value |= _data[_offset++] << 8;
			value |= _data[_offset++];
			return value;
		}

		public long ReadLong() {
			if (_offset + 8 > _length) return 0;
			var value = (long)_data[_offset++] << 56;
			value |= (long)_data[_offset++] << 48;
			value |= (long)_data[_offset++] << 40;
			value |= (long)_data[_offset++] << 32;
			value |= (long)_data[_offset++] << 24;
			value |= (long)_data[_offset++] << 16;
			value |= (long)_data[_offset++] << 8;
			value |= _data[_offset++];
			return value;
		}

		public float ReadFloat() {
			if (_offset + 4 > _length) return 0;
			var bytes = new byte[4];
			for (var i = 0; i < 4; i++)
				bytes[i] = _data[_offset++];
			if (BitConverter.IsLittleEndian) Array.Reverse(bytes);
			return BitConverter.ToSingle(bytes, 0);
		}

		public double ReadDouble() {
			if (_offset + 8 > _length) return 0;
			var bytes = new byte[8];
			for (var i = 0; i < 8; i++)
				bytes[i] = _data[_offset++];
			if (BitConverter.IsLittleEndian) Array.Reverse(bytes);
			return BitConverter.ToDouble(bytes, 0);
		}

		public Vector3 ReadVector3() {
			if (_offset + 12 > _length) return Vector3.Zero;
			var x = ReadFloat();
			var y = ReadFloat();
			var z = ReadFloat();
			return new Vector3(x, y, z);
		}

		public Quaternion ReadQuaternion() {
			if (_offset + 16 > _length) return Quaternion.Identity;
			var x = ReadFloat();
			var y = ReadFloat();
			var z = ReadFloat();
			var w = ReadFloat();
			return new Quaternion(x, y, z, w);
		}

		public byte[] ReadBytes(ushort length) {
			if (_offset + length > _length) return [];
			var value = new byte[length];
			Array.Copy(_data, _offset, value, 0, length);
			_offset += length;
			return value;
		}

		public uint ReadUInt()
			=> (uint)ReadInt();

		public Buffer Clone(ushort os = 0, ushort len = 0) {
			var buffer = new Buffer(new byte[len == 0 ? _data.Length : len]) { _offset = os };
			Array.Copy(_data, buffer._data, len == 0 ? _data.Length : len);
			buffer._offset = _offset;
			buffer._length = len == 0 ? _length : len;
			return buffer;
		}

		public void Clear() {
			_offset = 0;
			_length = 0;
		}

		public int Remaining()
			=> _length - _offset;

		public bool Write(Buffer buffer) {
			if (_offset + buffer._length > _data.Length) return false;
			for (var i = 0; i < buffer._length; i++)
				_data[_offset++] = buffer._data[i];
			if (_offset > _length) _length = _offset;
			return true;
		}

		public T? Read<T>() {
			if (typeof(T) == typeof(byte)) return (T)(object)ReadByte();
			if (typeof(T) == typeof(ushort)) return (T)(object)ReadUShort();
			if (typeof(T) == typeof(string)) {
				var value = ReadString();
				return value != null ? (T)(object)value : default;
			}

			if (typeof(T) == typeof(DateTime)) return (T)(object)ReadDateTime();
			if (typeof(T) == typeof(int)) return (T)(object)ReadInt();
			if (typeof(T) == typeof(long)) return (T)(object)ReadLong();
			if (typeof(T) == typeof(float)) return (T)(object)ReadFloat();
			if (typeof(T) == typeof(double)) return (T)(object)ReadDouble();
			if (typeof(T) == typeof(Vector3)) return (T)(object)ReadVector3();
			if (typeof(T) == typeof(Quaternion)) return (T)(object)ReadQuaternion();
			if (typeof(T) == typeof(byte[])) {
				var value = ReadBytes(ReadUShort());
				return value != null ? (T)(object)value : default;
			}

			if (typeof(T) == typeof(uint)) return (T)(object)ReadUInt();
			if (typeof(T) == typeof(Buffer)) return (T)(object)Clone();
			return default;
		}

		public T? ReadEnum<T>() where T : Enum {
			var type = Enum.GetUnderlyingType(typeof(T));
			if (type == typeof(byte)) return (T)(object)ReadByte();
			if (type == typeof(ushort)) return (T)(object)ReadUShort();
			if (type == typeof(uint)) return (T)(object)ReadUInt();
			return default;
		}

		public bool Write(Enum value) {
			var type = Enum.GetUnderlyingType(value.GetType());
			if (type == typeof(byte)) return Write(Convert.ToByte(value));
			if (type == typeof(ushort)) return Write(Convert.ToUInt16(value));
			if (type == typeof(uint)) return Write(Convert.ToUInt32(value));
			return false;
		}
	}
}