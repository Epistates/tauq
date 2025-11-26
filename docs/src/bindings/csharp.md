# C# / .NET Bindings

## Installation

Install via NuGet (Package name TBD).

## Usage

```csharp
using Tauq;

class Program {
    static void Main() {
        // Note: !def implies !use, so data rows immediately follow
        string input = "!def User id name\n1 Alice\n2 Bob";

        // 1. Parse to JSON String
        string json = TauqInterop.ToJson(input);
        Console.WriteLine(json);
        // Use System.Text.Json to deserialize 'json'

        // 2. Format
        string tqn = TauqInterop.ToTauq("[{\"id\": 1, \"name\": \"Alice\"}]");
        Console.WriteLine(tqn);

        // 3. Exec Query
        string res = TauqInterop.ExecQuery("!emit echo '1 Alice'", true); // safeMode=true

        // 4. Minify
        string min = TauqInterop.Minify("!def T x; 1; 2; 3");
        Console.WriteLine(min);
    }
}
```