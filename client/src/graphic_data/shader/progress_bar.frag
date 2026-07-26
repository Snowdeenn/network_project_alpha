#version 330

in vec2 fragTexCoord;
in vec4 fragColor;

uniform sampler2D texture0;
uniform float u_ratio;   // 0.0 = presque mort, 1.0 = plein

out vec4 finalColor;

void main()
{
    // vert quand ratio=1.0, rouge quand ratio=0.0
    vec3 green = vec3(0.0, 1.0, 0.0);
    vec3 red   = vec3(1.0, 0.0, 0.0);
    vec3 color = mix(red, green, u_ratio);
    
    finalColor = vec4(color, 1.0) * fragColor;
}