inspect_asm::alloc_iter_u32_bump_vec::down:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	movups xmm0, xmmword ptr [rip + .L__unnamed_0]
	movaps xmmword ptr [rsp], xmm0
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdi
	test rdx, rdx
	jne .LBB0_5
	mov rsi, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 8]
.LBB0_0:
	mov rcx, qword ptr [rsp + 16]
	test rcx, rcx
	je .LBB0_1
	mov rbx, qword ptr [rsp + 24]
	lea r8, [rsi + 4*rdx]
	lea rax, [4*rdx]
	lea rdi, [rsi + 4*rcx]
	sub rdi, rax
	cmp rdi, r8
	jae .LBB0_2
	mov rax, rsi
	jmp .LBB0_3
.LBB0_1:
	mov eax, 4
	xor edx, edx
	jmp .LBB0_4
.LBB0_2:
	mov r14, rdx
	mov rdx, rax
	mov r15, rdi
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, r15
	mov rdx, r14
.LBB0_3:
	mov rcx, qword ptr [rbx]
	mov qword ptr [rcx], rax
.LBB0_4:
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_5:
	mov rbx, rdx
	mov r14, rsp
	mov rdi, r14
	mov r15, rsi
	mov rsi, rdx
	call bump_scope::mut_bump_vec::MutBumpVec<T,A,_,_,_>::generic_grow_cold
	mov rax, r15
	mov rdx, qword ptr [rsp + 8]
	shl rbx, 2
	xor r12d, r12d
	jmp .LBB0_7
.LBB0_6:
	mov rsi, qword ptr [rsp]
	mov dword ptr [rsi + 4*rdx], ebp
	inc rdx
	mov qword ptr [rsp + 8], rdx
	add r12, 4
	cmp rbx, r12
	je .LBB0_0
.LBB0_7:
	mov ebp, dword ptr [rax + r12]
	cmp qword ptr [rsp + 16], rdx
	jne .LBB0_6
	mov esi, 1
	mov rdi, r14
	call bump_scope::mut_bump_vec::MutBumpVec<T,A,_,_,_>::generic_grow_cold
	mov rax, r15
	mov rdx, qword ptr [rsp + 8]
	jmp .LBB0_6
