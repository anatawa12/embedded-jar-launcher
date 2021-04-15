import java.io.File;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.Arrays;

public class NativeWrapperRunner {
    public static void main(String[] args) throws Throwable {
        String options = args[0];
        boolean debug = options.contains("-debug ");
        File jar = new File(args[1]);
        jar.deleteOnExit();
        if (debug) System.err.printf("deleteOnExit: %s%n", jar);
        String mainClass = args[2];
        Class<?> clazz = Class.forName(mainClass);
        if (debug) System.err.printf("loaded class: %s: %s%n", mainClass, clazz);
        Method method = clazz.getDeclaredMethod("main", String[].class);
        method.setAccessible(true);
        try {
            if (debug) System.err.printf("calling main method: %s%n", method);
            method.invoke(null, (Object) Arrays.copyOfRange(args, 3, args.length));
        } catch (InvocationTargetException e) {
            throw e.getTargetException();
        } finally {
            try {
                if (debug) System.err.printf("deleting: %s%n", jar);
                //noinspection ResultOfMethodCallIgnored
                jar.delete();
            } catch (Throwable ignored) {
            }
        }
    }
}
