inspect_asm::alloc_fmt::up_a:
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
	movups xmm0, xmmword ptr [rip + .L__unnamed_1]
	movaps xmmword ptr [rsp], xmm0
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdi
	lea rsi, [rip + .L__unnamed_2]
	mov rdi, rsp
	lea rdx, [rsp + 72]
	call qword ptr [rip + core::fmt::write@GOTPCREL]
	test al, al
	jne .LBB0_1
	mov rax, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 8]
	mov rcx, qword ptr [rsp + 24]
	mov rsi, qword ptr [rsp + 16]
	add rsi, rax
	mov rcx, qword ptr [rcx]
	cmp rsi, qword ptr [rcx]
	je .LBB0_0
	add rsp, 120
	ret
.LBB0_0:
	lea rsi, [rax + rdx]
	add rsi, 3
	and rsi, -4
	mov qword ptr [rcx], rsi
	add rsp, 120
	ret
.LBB0_1:
	call qword ptr [rip + bump_scope::private::format_trait_error@GOTPCREL]
