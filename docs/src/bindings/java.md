# Java Bindings

## Installation

Add the dependency to your build tool (Maven/Gradle).

## Usage

```java
import com.tauq.Tauq;

public class Main {
    public static void main(String[] args) {
        // Note: !def implies !use, so data rows immediately follow
        String input = "!def User id name\n1 Alice\n2 Bob";

        // 1. Parse to JSON String
        String json = Tauq.parseToJson(input);
        System.out.println(json);
        // Use Jackson/Gson to parse 'json' string into POJOs

        // 2. Format JSON to Tauq
        String tqn = Tauq.formatJson("[{\"id\": 1, \"name\": \"Alice\"}]");
        System.out.println(tqn);

        // 3. Exec Query
        String res = Tauq.execQuery("!emit echo '1 Alice'", true); // safeMode=true

        // 4. Minify
        String min = Tauq.minify("!def T x; 1; 2; 3");
        System.out.println(min);
    }
}
```