inspect_asm::alloc_fmt::mut_up_a:
	sub rsp, 120
	mov qword ptr [rsp + 40], rsi
	mov qword ptr [rsp + 48], rdx
	lea rax, [rsp + 40]
	mov qword ptr [rsp + 56], rax
	lea rax, [rip + <&T as core::fmt::Display>::fmt]
	mov qword ptr [rsp + 64], rax
	lea rax, [rip + .L__unnamed_0]
	mov qword ptr [rsp + 72], rax
	mov qword ptr [rsp + 80], 2
	mov qword ptr [rsp + 104], 0
	lea rax, [rsp + 56]
	mov qword ptr [rsp + 88], rax
	mov qword ptr [rsp + 96], 1
	xorps xmm0, xmm0
	movups xmmword ptr [rsp + 16], xmm0
	mov qword ptr [rsp + 8], 1
	mov qword ptr [rsp + 32], rdi
	lea rsi, [rip + .L__unnamed_1]
	lea rdi, [rsp + 8]
	lea rdx, [rsp + 72]
	call qword ptr [rip + core::fmt::write@GOTPCREL]
	test al, al
	jne .LBB_3
	cmp qword ptr [rsp + 24], 0
	je .LBB_2
	mov rcx, qword ptr [rsp + 32]
	mov rax, qword ptr [rsp + 8]
	mov rdx, qword ptr [rsp + 16]
	mov rcx, qword ptr [rcx]
	lea rsi, [rax + rdx]
	add rsi, 3
	and rsi, -4
	mov qword ptr [rcx], rsi
	add rsp, 120
	ret
.LBB_2:
	mov eax, 1
	xor edx, edx
	add rsp, 120
	ret
.LBB_3:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]