plugins {
    `embedded-kotlin`
    `kotlin-dsl`
    `java-gradle-plugin`
}

group = project(":").group
version = project(":").version

repositories {
    mavenCentral()
}

@Suppress("SuspiciousCollectionReassignment")
tasks.compileKotlin {
    kotlinOptions {
        freeCompilerArgs += "-XXLanguage:+TrailingCommas"
    }
}

gradlePlugin {
    plugins {
        create("gradle-rust") {
            id = "com.anatawa12.gradle-rust"
            implementationClass = "com.anatawa12.gradleRust.GradlePlugin"
        }
    }
}

dependencies {
    implementation(kotlin("stdlib"))
}
