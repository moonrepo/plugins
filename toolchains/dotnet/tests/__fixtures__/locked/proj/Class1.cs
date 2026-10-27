namespace Locked;

public static class Class1
{
    public static string Json() => Newtonsoft.Json.JsonConvert.SerializeObject(new { ok = true });
}
