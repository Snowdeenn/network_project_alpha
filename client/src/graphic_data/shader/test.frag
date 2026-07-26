#version 330

in vec2 fragTexCoord;
in vec4 fragColor;

uniform sampler2D texture0;

out vec4 finalColor;

void main()
{
    // Juste la couleur passée en paramètre, sans toucher à la texture
    finalColor = fragColor;
}