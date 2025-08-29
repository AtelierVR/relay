namespace Relay.Players;

internal enum PlayerRig : ushort
{
    // position on floor
    Base = 0,

    // postions of body
    Hips = 1,
    Spine = 2,
    Chest = 3,
    Neck = 4,
    Head = 5,

    // positions of arms
    LeftShoulder = 6,
    LeftArm = 7,
    LeftForearm = 8,
    LeftHand = 9,
    RightShoulder = 10,
    RightArm = 11,
    RightForearm = 12,
    RightHand = 13,

    // positions of legs
    LeftUpperLeg = 14,
    LeftLowerLeg = 15,
    LeftFoot = 16,
    LeftToes = 17,
    RightUpperLeg = 18,
    RightLowerLeg = 19,
    RightFoot = 20,
    RightToes = 21,

    // positions of fingers
    LeftThumb = 22,
    LeftIndex = 23,
    LeftMiddle = 24,
    LeftRing = 25,
    LeftPinky = 26,
    LeftThumbTip = 27,
    LeftIndexTip = 28,
    LeftMiddleTip = 29,
    LeftRingTip = 30,
    LeftPinkyTip = 31,
    LeftThumbNail = 32,
    LeftIndexNail = 33,
    LeftMiddleNail = 34,
    LeftRingNail = 35,
    LeftPinkyNail = 36,
    RightThumb = 37,
    RightIndex = 38,
    RightMiddle = 39,
    RightRing = 40,
    RightPinky = 41,
    RightThumbTip = 42,
    RightIndexTip = 43,
    RightMiddleTip = 44,
    RightRingTip = 45,
    RightPinkyTip = 46,
    RightThumbNail = 47,
    RightIndexNail = 48,
    RightMiddleNail = 49,
    RightRingNail = 50,
    RightPinkyNail = 51,

    // positions of head parts
    RightEye = 52,
    LeftEye = 53
}